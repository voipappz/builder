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
    body::Body,
    extract::Request,
    http::{header, HeaderValue, Response, StatusCode},
    middleware::{self, Next},
    routing::get,
    Router,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::{debug, info};
use uuid::Uuid;

use crate::ws::acp::{AcpChannelState, AgentPolicy};
use crate::ws::mcp::McpChannelState;
use crate::ws::WsState;

pub const SECRET_HEADER: &str = "x-builder-secret";

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
    let policy = AgentPolicy {
        command: config.agent_command.clone(),
        env: config.agent_env.clone(),
        cwd: config.workspace.to_string_lossy().into_owned(),
        mcp_servers: config.mcp_servers.clone(),
    };
    let ws_state = WsState::new()
        .with_acp_state(AcpChannelState::new().with_policy(policy))
        .with_mcp_state(McpChannelState::new())
        .acp_and_mcp_only();

    let page_config = config.clone();
    let ws_routes = Router::new()
        .route("/socket/websocket", get(crate::ws::ws_handler))
        .with_state(ws_state.clone());

    let router = Router::new()
        .route(
            "/",
            get(move || {
                let config = page_config.clone();
                async move { stub_page(&config) }
            }),
        )
        .merge(ws_routes)
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
        api_mcp: !config.mcp_servers.is_empty(),
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
