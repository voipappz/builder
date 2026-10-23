//! VoipAppz Builder's local server.
//!
//! Serves two routes and nothing else:
//!
//! * `GET /` — the stub page. It carries local facts in a meta tag and loads
//!   the chat client from the configured client address (the VoipAppz app),
//!   from one hosted script.
//! * `GET /socket/websocket` — the Phoenix websocket, limited to the ACP and
//!   MCP channels. The agent, its working directory and its MCP servers come
//!   from [`AgentPolicy`], never from the browser.
//!
//! Every request must carry the per-launch secret (`?secret=` or the
//! `x-builder-secret` header), a Host naming this server, and, when present,
//! an Origin naming this server. The upstream file, shell and proxy routes
//! are not mounted.

use axum::{
    body::{Body, Bytes},
    extract::{Request, State},
    http::{header, HeaderValue, Response, StatusCode},
    middleware::{self, Next},
    routing::{get, post},
    Router,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

use crate::ws::acp::{AcpChannelState, AgentPolicy};
use crate::ws::mcp::McpChannelState;
use crate::ws::WsState;

pub const SECRET_HEADER: &str = "x-builder-secret";
const API_MCP_PATH: &str = "/mcp/api";

/// The VoipAppz API's MCP server, reached only through Builder. The agent
/// writes code only in the app, so the API is read-only: tools that change
/// the platform are hidden and refused, and the login stays in Builder.
#[derive(Debug, Clone)]
pub struct ApiMcp {
    /// Full address, e.g. `http://127.0.0.1:5055/api/mcp`.
    pub url: String,
    /// Basic credential (base64 of `email:password`).
    pub basic: String,
}

const WRITE_VERBS: &[&str] = &[
    "create", "update", "delete", "destroy", "remove", "add", "set", "buy", "send",
    "start", "stop", "cancel", "transfer", "hangup", "enable", "disable", "provision",
];

/// Whether an MCP tool changes the platform: marked destructive, or named
/// for a change (`calls.create`, `numbers.buy`).
pub fn tool_writes(tool: &Value) -> bool {
    let destructive = tool
        .pointer("/annotations/destructiveHint")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let name = tool.get("name").and_then(Value::as_str).unwrap_or("");
    destructive || name_writes(name)
}

fn name_writes(name: &str) -> bool {
    name.rsplit(['.', '_', '/'])
        .next()
        .is_some_and(|verb| WRITE_VERBS.iter().any(|w| verb.eq_ignore_ascii_case(w)))
}

struct ApiProxy {
    api: ApiMcp,
    client: reqwest::Client,
    /// Write tools seen in the API's tools/list.
    blocked: Mutex<Option<HashSet<String>>>,
}

impl ApiProxy {
    async fn forward(&self, body: Bytes) -> Result<(StatusCode, Bytes), String> {
        let response = self
            .client
            .post(&self.api.url)
            .header(header::AUTHORIZATION, format!("Basic {}", self.api.basic))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("API unreachable: {}", e))?;
        let status = StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        Ok((status, bytes))
    }

    async fn learn(&self, tools: &[Value]) {
        let names = tools
            .iter()
            .filter(|t| tool_writes(t))
            .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_string))
            .collect();
        *self.blocked.lock().await = Some(names);
    }

    async fn is_write(&self, name: &str) -> bool {
        if name_writes(name) {
            return true;
        }
        if self.blocked.lock().await.is_none() {
            let list = serde_json::json!({"jsonrpc": "2.0", "id": "builder-tools", "method": "tools/list"});
            if let Ok((_, bytes)) = self.forward(Bytes::from(list.to_string())).await {
                let tools = serde_json::from_slice::<Value>(&bytes)
                    .ok()
                    .and_then(|v| v.pointer("/result/tools").and_then(Value::as_array).cloned())
                    .unwrap_or_default();
                self.learn(&tools).await;
            }
        }
        self.blocked
            .lock()
            .await
            .as_ref()
            .map_or(true, |blocked| blocked.contains(name))
    }
}

fn json_response(status: StatusCode, body: impl Into<Body>) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.into())
        .expect("static response")
}

async fn api_mcp_proxy(State(proxy): State<Arc<ApiProxy>>, body: Bytes) -> Response<Body> {
    let message: Value = match serde_json::from_slice(&body) {
        Ok(Value::Object(map)) => Value::Object(map),
        _ => return json_response(StatusCode::BAD_REQUEST, r#"{"error":"one JSON-RPC message per request"}"#),
    };
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("").to_string();

    if method == "tools/call" {
        let name = message.pointer("/params/name").and_then(Value::as_str).unwrap_or("");
        if proxy.is_write(name).await {
            let refusal = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "isError": true,
                    "content": [{
                        "type": "text",
                        "text": format!("{} changes the VoipAppz platform. In Builder the API is read-only: change the app's code instead.", name)
                    }]
                }
            });
            return json_response(StatusCode::OK, refusal.to_string());
        }
    }

    let (status, bytes) = match proxy.forward(body).await {
        Ok(r) => r,
        Err(e) => {
            let error = serde_json::json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32603, "message": e}});
            return json_response(StatusCode::BAD_GATEWAY, error.to_string());
        }
    };
    if method == "tools/list" && status.is_success() {
        if let Ok(mut reply) = serde_json::from_slice::<Value>(&bytes) {
            if let Some(tools) = reply.pointer_mut("/result/tools").and_then(Value::as_array_mut) {
                proxy.learn(tools).await;
                tools.retain(|t| !tool_writes(t));
            }
            return json_response(status, reply.to_string());
        }
    }
    if bytes.is_empty() {
        return Response::builder().status(status).body(Body::empty()).expect("static response");
    }
    json_response(status, bytes)
}

#[derive(Debug, Clone)]
pub struct BuilderConfig {
    pub port: u16,
    pub secret: String,
    pub workspace: PathBuf,
    /// Origin the chat client is loaded from, e.g. `https://switch.voipappz.io`
    /// or `http://localhost:3000` for a local app run.
    pub client_url: String,
    pub agent_command: String,
    pub agent_env: HashMap<String, String>,
    /// MCP servers handed to the agent on every session (ACP `mcpServers`).
    pub mcp_servers: Vec<Value>,
    /// The VoipAppz API's MCP server, offered read-only through Builder.
    pub api_mcp: Option<ApiMcp>,
    /// Shown to the user; never includes credentials.
    pub api_url: Option<String>,
    pub preview_url: String,
}

impl BuilderConfig {
    /// A random per-launch secret.
    pub fn new_secret() -> String {
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
    }

    fn self_hosts(&self) -> [String; 2] {
        [
            format!("127.0.0.1:{}", self.port),
            format!("localhost:{}", self.port),
        ]
    }

    fn client_origin(&self) -> String {
        self.client_url.trim_end_matches('/').to_string()
    }

    /// A local app run (Vite dev server) rather than a deployed client.
    fn client_is_dev_server(&self) -> bool {
        let rest = self
            .client_origin()
            .strip_prefix("http://")
            .map(str::to_string);
        match rest {
            Some(host) => {
                let host = host.split(':').next().unwrap_or_default().to_string();
                host == "localhost" || host == "127.0.0.1"
            }
            None => false,
        }
    }

    /// Admin and User previews get their own origins, so their logins
    /// (kept in localStorage per origin) stay apart.
    pub fn preview_urls(&self) -> (String, String) {
        let base = self.preview_url.trim_end_matches('/');
        if let Some(rest) = base.strip_prefix("http://localhost") {
            (
                format!("http://admin.localhost{}", rest),
                format!("http://user.localhost{}", rest),
            )
        } else {
            (base.to_string(), base.to_string())
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo<'a> {
    version: &'a str,
    secret: &'a str,
    socket_path: &'a str,
    workspace: String,
    workspace_name: String,
    client_url: String,
    api_url: Option<&'a str>,
    api_mcp: bool,
    agent: &'a str,
    previews: Previews,
}

#[derive(Serialize)]
struct Previews {
    admin: String,
    user: String,
}

pub fn router(config: BuilderConfig) -> Router {
    router_with_state(config).0
}

fn router_with_state(config: BuilderConfig) -> (Router, WsState) {
    let mut mcp_servers = config.mcp_servers.clone();
    if config.api_mcp.is_some() {
        mcp_servers.push(serde_json::json!({
            "type": "http",
            "name": "voipappz",
            "url": format!("http://127.0.0.1:{}{}", config.port, API_MCP_PATH),
            "headers": [{"name": SECRET_HEADER, "value": config.secret}]
        }));
    }
    let policy = AgentPolicy {
        command: config.agent_command.clone(),
        env: config.agent_env.clone(),
        cwd: config.workspace.to_string_lossy().into_owned(),
        mcp_servers,
    };
    let ws_state = WsState::new()
        .with_acp_state(AcpChannelState::new().with_policy(policy))
        .with_mcp_state(McpChannelState::new())
        .acp_and_mcp_only();

    let page_config = config.clone();
    let ws_routes = Router::new()
        .route("/socket/websocket", get(crate::ws::ws_handler))
        .with_state(ws_state.clone());

    let mut api_routes = Router::new();
    if let Some(api) = config.api_mcp.clone() {
        let proxy = Arc::new(ApiProxy {
            api,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("http client"),
            blocked: Mutex::new(None),
        });
        api_routes = Router::new()
            .route(API_MCP_PATH, post(api_mcp_proxy))
            .with_state(proxy);
    }

    let router = Router::new()
        .route(
            "/",
            get(move || {
                let config = page_config.clone();
                async move { stub_page(&config) }
            }),
        )
        .merge(ws_routes)
        .merge(api_routes)
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let config = config.clone();
            async move { guard(&config, req, next).await }
        }));
    (router, ws_state)
}

pub async fn serve(
    config: BuilderConfig,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve_on(listener, config, shutdown).await
}

/// Serve on a listener the caller already bound (to 127.0.0.1).
pub async fn serve_on(
    listener: tokio::net::TcpListener,
    config: BuilderConfig,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    info!("VoipAppz Builder listening on http://{}", listener.local_addr()?);
    let (router, ws_state) = router_with_state(config);
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await;
    // Stop the agents this server started; each runs in its own process
    // group, which the exit monitor kills as a whole.
    ws_state.clear().await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    result?;
    Ok(())
}

async fn guard(config: &BuilderConfig, req: Request, next: Next) -> Response<Body> {
    let hosts = config.self_hosts();
    let host_ok = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| hosts.iter().any(|allowed| allowed == h));
    if !host_ok {
        debug!("Rejected request: unexpected Host");
        return status(StatusCode::FORBIDDEN);
    }

    if let Some(origin) = req.headers().get(header::ORIGIN) {
        let origin_ok = origin.to_str().ok().is_some_and(|o| {
            hosts
                .iter()
                .any(|allowed| o == format!("http://{}", allowed))
        });
        if !origin_ok {
            debug!("Rejected request: unexpected Origin");
            return status(StatusCode::FORBIDDEN);
        }
    }

    let presented = req
        .headers()
        .get(SECRET_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(str::to_string)
        .or_else(|| query_param(req.uri().query(), "secret"));
    let secret_ok = presented.is_some_and(|p| constant_time_eq(p.as_bytes(), config.secret.as_bytes()));
    if !secret_ok {
        debug!("Rejected request: missing or wrong launch secret");
        return status(StatusCode::UNAUTHORIZED);
    }

    next.run(req).await
}

fn status(code: StatusCode) -> Response<Body> {
    Response::builder()
        .status(code)
        .body(Body::empty())
        .expect("static response")
}

fn query_param(query: Option<&str>, name: &str) -> Option<String> {
    query?.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn stub_page(config: &BuilderConfig) -> Response<Body> {
    let (admin, user) = config.preview_urls();
    let client = config.client_origin();
    let info = PageInfo {
        version: env!("CARGO_PKG_VERSION"),
        secret: &config.secret,
        socket_path: "/socket/websocket",
        workspace: config.workspace.to_string_lossy().into_owned(),
        workspace_name: config
            .workspace
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        client_url: client.clone(),
        api_url: config.api_url.as_deref(),
        api_mcp: config.api_mcp.is_some(),
        agent: "Claude Code",
        previews: Previews {
            admin: admin.clone(),
            user: user.clone(),
        },
    };
    let info_json = serde_json::to_string(&info).expect("page info serializes");
    let nonce = Uuid::new_v4().simple().to_string();

    // A local app run is a Vite dev server: load its client and the React
    // refresh preamble, then the entry module. A deployed app serves one
    // built script.
    let (preamble, entry) = if config.client_is_dev_server() {
        (
            format!(
                r#"<script type="module" nonce="{nonce}">
      import RefreshRuntime from "{client}/@react-refresh";
      RefreshRuntime.injectIntoGlobalHook(window);
      window.$RefreshReg$ = () => {{}};
      window.$RefreshSig$ = () => (type) => type;
      window.__vite_plugin_react_preamble_installed__ = true;
    </script>
    <script type="module" nonce="{nonce}" src="{client}/@vite/client"></script>"#
            ),
            format!("{client}/src/builder-client/main.jsx"),
        )
    } else {
        (String::new(), format!("{client}/builder/client.js"))
    };

    let html = format!(
        r##"<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="voipappz-builder:info" content="{info}" />
    <title>VoipAppz Builder</title>
    {preamble}
    <script nonce="{nonce}">
      (function () {{
        var src = {entry_json};
        var script = document.createElement("script");
        script.type = "module";
        script.nonce = "{nonce}";
        script.src = src;
        script.onerror = function () {{
          document.getElementById("root").innerHTML =
            '<div style="font-family:sans-serif;padding:2rem">' +
            "<h2>VoipAppz Builder could not load the chat</h2>" +
            "<p>Tried: <code></code></p>" +
            "<p>Start the app (npm run dev in the app checkout) or set --client-url.</p></div>";
          document.querySelector("#root code").textContent = src;
        }};
        document.head.appendChild(script);
      }})();
    </script>
  </head>
  <body>
    <div id="root"></div>
  </body>
</html>
"##,
        info = html_escape(&info_json),
        entry_json = serde_json::to_string(&entry).expect("string serializes"),
    );

    let ws_self = config
        .self_hosts()
        .iter()
        .map(|h| format!("ws://{}", h))
        .collect::<Vec<_>>()
        .join(" ");
    let client_ws = client
        .replacen("http://", "ws://", 1)
        .replacen("https://", "wss://", 1);
    let csp = format!(
        "default-src 'none'; \
         script-src 'nonce-{nonce}' {client}; \
         style-src 'unsafe-inline' {client}; \
         img-src 'self' data: {client}; \
         font-src data: {client}; \
         connect-src 'self' {ws_self} {client} {client_ws}; \
         frame-src {admin} {user}; \
         base-uri 'none'; form-action 'none'"
    );

    let mut response = Response::new(Body::from(html));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    if let Ok(value) = HeaderValue::from_str(&csp) {
        headers.insert(header::CONTENT_SECURITY_POLICY, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt;

    fn config() -> BuilderConfig {
        BuilderConfig {
            port: 9832,
            secret: "s3cret".to_string(),
            workspace: PathBuf::from("/tmp/app-builder"),
            client_url: "http://localhost:3000".to_string(),
            agent_command: "true".to_string(),
            agent_env: HashMap::new(),
            mcp_servers: vec![],
            api_mcp: None,
            api_url: None,
            preview_url: "http://localhost:3000".to_string(),
        }
    }

    async fn get(uri: &str, headers: &[(&str, &str)]) -> Response<Body> {
        let mut req = Request::builder().uri(uri).header("host", "127.0.0.1:9832");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        router(config())
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn page_requires_the_secret() {
        assert_eq!(get("/", &[]).await.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(get("/?secret=wrong", &[]).await.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(get("/?secret=s3cret", &[]).await.status(), StatusCode::OK);
        assert_eq!(get("/", &[(SECRET_HEADER, "s3cret")]).await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_foreign_host_and_origin() {
        let req = Request::builder()
            .uri("/?secret=s3cret")
            .header("host", "evil.example:9832")
            .body(Body::empty())
            .unwrap();
        let res = router(config()).oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        let res = get("/?secret=s3cret", &[("origin", "http://evil.example")]).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let res = get("/?secret=s3cret", &[("origin", "http://127.0.0.1:9832")]).await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn upstream_file_shell_and_proxy_routes_are_absent() {
        for path in ["/read", "/write", "/shell", "/proxy", "/download", "/socket/mcp-remote-client"] {
            let res = get(&format!("{}?secret=s3cret", path), &[]).await;
            assert_eq!(res.status(), StatusCode::NOT_FOUND, "{}", path);
        }
    }

    #[tokio::test]
    async fn page_loads_the_dev_client_and_carries_info() {
        let res = get("/?secret=s3cret", &[]).await;
        let csp = res
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(csp.contains("script-src 'nonce-"));
        assert!(csp.contains("frame-src http://admin.localhost:3000 http://user.localhost:3000"));
        let body = String::from_utf8(to_bytes(res.into_body(), usize::MAX).await.unwrap().to_vec()).unwrap();
        assert!(body.contains("http://localhost:3000/@react-refresh"));
        assert!(body.contains("\"http://localhost:3000/src/builder-client/main.jsx\""));
        assert!(body.contains("voipappz-builder:info"));
        assert!(body.contains("&quot;workspaceName&quot;:&quot;app-builder&quot;"));
    }

    #[test]
    fn deployed_client_loads_one_script() {
        let mut c = config();
        c.client_url = "https://switch.voipappz.io/".to_string();
        assert!(!c.client_is_dev_server());
        let res = stub_page(&c);
        let csp = res.headers().get(header::CONTENT_SECURITY_POLICY).unwrap().to_str().unwrap();
        assert!(csp.contains("https://switch.voipappz.io"));
    }

    #[test]
    fn write_tools_are_recognised() {
        use serde_json::json;
        assert!(tool_writes(&json!({"name": "calls.create"})));
        assert!(tool_writes(&json!({"name": "numbers.buy"})));
        assert!(tool_writes(&json!({"name": "nodes.inspect", "annotations": {"destructiveHint": true}})));
        assert!(!tool_writes(&json!({"name": "calls.history"})));
        assert!(!tool_writes(&json!({"name": "devices.list"})));
        assert!(!tool_writes(&json!({"name": "logs.search", "annotations": {"readOnlyHint": true}})));
    }

    #[test]
    fn agent_may_write_only_in_the_workspace() {
        use serde_json::json;
        let root = std::env::temp_dir().join(format!("builder-ws-{}", std::process::id()));
        let outside = std::env::temp_dir().join(format!("builder-out-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        let policy = AgentPolicy {
            command: "agent".into(),
            env: HashMap::new(),
            cwd: root.to_string_lossy().into_owned(),
            mcp_servers: vec![],
        };
        let ask = |kind: &str, path: &str| {
            json!({
                "sessionId": "s",
                "toolCall": {"toolCallId": "t", "kind": kind, "locations": [{"path": path}]},
                "options": [
                    {"optionId": "allow", "name": "Allow", "kind": "allow_once"},
                    {"optionId": "no", "name": "Reject", "kind": "reject_once"}
                ]
            })
        };
        let inside = root.join("src/App.jsx").to_string_lossy().into_owned();
        assert_eq!(policy.refuse_permission(&ask("edit", &inside)), None);
        assert_eq!(policy.refuse_permission(&ask("edit", "src/new-file.jsx")), None);

        let refused = json!({"outcome": {"outcome": "selected", "optionId": "no"}});
        let api = outside.join("lib/x.rb").to_string_lossy().into_owned();
        assert_eq!(policy.refuse_permission(&ask("edit", &api)), Some(refused.clone()));
        assert_eq!(policy.refuse_permission(&ask("delete", "../voipappz-api/Gemfile")), Some(refused.clone()));
        assert_eq!(policy.refuse_permission(&ask("edit", "src/../../escape.js")), Some(refused.clone()));
        assert_eq!(policy.refuse_permission(&ask("edit", ".git/hooks/pre-commit")), Some(refused.clone()));
        #[cfg(unix)]
        assert_eq!(policy.refuse_permission(&ask("edit", "link/x.js")), Some(refused.clone()));
        // A write with no named target is refused; reads and commands go to the user.
        let unnamed = json!({"toolCall": {"kind": "edit"}, "options": []});
        assert_eq!(policy.refuse_permission(&unnamed), Some(json!({"outcome": {"outcome": "cancelled"}})));
        assert_eq!(policy.refuse_permission(&ask("read", &api)), None);
        assert_eq!(policy.refuse_permission(&ask("execute", &api)), None);

        std::fs::remove_dir_all(&root).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    #[tokio::test]
    async fn api_tools_are_read_only_through_builder() {
        use axum::extract::State as AxState;
        use serde_json::json;
        // A fake VoipAppz API that records what reaches it.
        let calls: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        async fn fake(AxState(calls): AxState<Arc<Mutex<Vec<(String, String)>>>>, req: Request) -> Response<Body> {
            let auth = req.headers().get(header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("").to_string();
            let body = axum::body::to_bytes(req.into_body(), usize::MAX).await.unwrap();
            let msg: Value = serde_json::from_slice(&body).unwrap();
            let method = msg["method"].as_str().unwrap_or("").to_string();
            calls.lock().await.push((method.clone(), auth));
            let result = match method.as_str() {
                "tools/list" => json!({"tools": [
                    {"name": "calls.history"},
                    {"name": "calls.create", "annotations": {"destructiveHint": true}},
                    {"name": "nodes.reboot", "annotations": {"destructiveHint": true}}
                ]}),
                _ => json!({"content": [{"type": "text", "text": "ok"}]}),
            };
            json_response(StatusCode::OK, json!({"jsonrpc": "2.0", "id": msg["id"], "result": result}).to_string())
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().route("/api/mcp", post(fake)).with_state(calls.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let mut c = config();
        c.api_mcp = Some(ApiMcp { url: format!("http://127.0.0.1:{}/api/mcp", port), basic: "dXNlcjpwdw==".into() });
        let router = router(c);
        let call = |body: Value, secret: bool| {
            let router = router.clone();
            async move {
                let mut req = Request::builder()
                    .method("POST")
                    .uri("/mcp/api")
                    .header("host", "127.0.0.1:9832")
                    .header("content-type", "application/json");
                if secret {
                    req = req.header(SECRET_HEADER, "s3cret");
                }
                let res = router.oneshot(req.body(Body::from(body.to_string())).unwrap()).await.unwrap();
                let status = res.status();
                let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
                (status, serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null))
            }
        };

        let (status, _) = call(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}), false).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (_, listed) = call(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}), true).await;
        let names: Vec<_> = listed["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, vec!["calls.history"]);

        let (_, refused) = call(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "nodes.reboot"}}), true).await;
        assert_eq!(refused["result"]["isError"], true);
        let (_, refused) = call(json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "calls.create"}}), true).await;
        assert_eq!(refused["result"]["isError"], true);

        let (_, read) = call(json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "calls.history"}}), true).await;
        assert_eq!(read["result"]["content"][0]["text"], "ok");

        let seen = calls.lock().await.clone();
        assert!(seen.iter().all(|(_, auth)| auth == "Basic dXNlcjpwdw=="));
        assert_eq!(seen.iter().filter(|(m, _)| m == "tools/call").count(), 1, "write calls never reach the API");
    }

    #[test]
    fn policy_pins_cwd_and_mcp_servers() {
        use crate::ws::acp::JsonRpcRequest;
        let policy = AgentPolicy {
            command: "agent".into(),
            env: HashMap::new(),
            cwd: "/work".into(),
            mcp_servers: vec![serde_json::json!({"type": "http", "name": "voipappz"})],
        };
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            method: "session/new".into(),
            params: Some(serde_json::json!({"cwd": "/etc", "mcpServers": [{"name": "evil"}]})),
        };
        let pinned = policy.apply(&req);
        let params = pinned.params.unwrap();
        assert_eq!(params["cwd"], "/work");
        assert_eq!(params["mcpServers"][0]["name"], "voipappz");

        let prompt = JsonRpcRequest { method: "session/prompt".into(), ..req };
        assert_eq!(policy.apply(&prompt).params.unwrap()["cwd"], "/etc");
    }
}
