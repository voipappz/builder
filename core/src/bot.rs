//! A voice bot beside the app: a folder whose `builder.toml` says
//! `kind = "bot"`. Builder runs the bot's own `make setup` and `make dev`,
//! waits for its health address, restarts it after the agent changes it, and
//! bridges the page to its audio WebSocket with the bot's token, which the
//! page never sees.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use bot_ws::tungstenite::{client::IntoClientRequest, Message as UpMessage};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

use crate::command::{create_shell_command, spawn_command, ChildProcess};
use crate::launcher::{command_path, run_on, sh_quote, state_dir, wsl_env};

/// The `[builder]` table of a bot folder's `builder.toml`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BotSpec {
    pub kind: String,
    pub name: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_health")]
    pub health: String,
    #[serde(default = "default_socket")]
    pub socket: String,
    /// Present once `make setup` has run; setup runs when it is missing.
    #[serde(default)]
    pub setup_done: Option<String>,
    /// Seconds `make dev` may take to answer; the first run compiles.
    #[serde(default = "default_start_timeout")]
    pub start_timeout: u64,
}

fn default_port() -> u16 { 8090 }
fn default_health() -> String { "/healthz".into() }
fn default_socket() -> String { "/earshot".into() }
fn default_start_timeout() -> u64 { 300 }

#[derive(Deserialize)]
struct BuilderToml {
    builder: BotSpec,
}

/// The bot described by `dir/builder.toml`.
pub fn read_spec(dir: &Path) -> Result<BotSpec, String> {
    let path = dir.join("builder.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("{}: {e}. A bot folder has a builder.toml with kind = \"bot\".", path.display()))?;
    let spec = toml::from_str::<BuilderToml>(&text).map_err(|e| format!("{}: {e}", path.display()))?.builder;
    if spec.kind != "bot" {
        return Err(format!("{} says kind = \"{}\"; a bot folder says kind = \"bot\".", path.display(), spec.kind));
    }
    if !spec.health.starts_with('/') || !spec.socket.starts_with('/') {
        return Err(format!("{}: health and socket are paths starting with /", path.display()));
    }
    Ok(spec)
}

/// The shared token the bot asks of Earshot, from the bot's `.env`.
pub fn read_token(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(".env")).ok()?;
    text.lines()
        .find_map(|line| line.trim().strip_prefix("EARSHOT_BOT_TOKEN="))
        .map(|v| v.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|v| !v.is_empty())
}

/// What the bot's health address said.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Health {
    /// The process answers HTTP.
    pub up: bool,
    /// It answered 200: configured to take calls.
    pub ready: bool,
}

/// A running (or stopped) bot.
pub struct Bot {
    pub spec: BotSpec,
    /// The folder as this process sees it (a `\\wsl.localhost\…` path on Windows).
    pub dir: PathBuf,
    pub port: u16,
    distro: Option<String>,
    env: HashMap<String, String>,
    process: Mutex<Option<ChildProcess>>,
    /// One start at a time: a second restart waits for the first.
    starting: Mutex<()>,
}

impl std::fmt::Debug for Bot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bot").field("name", &self.spec.name).field("dir", &self.dir).field("port", &self.port).finish()
    }
}

impl Bot {
    pub fn new(spec: BotSpec, dir: PathBuf, port: u16, distro: Option<String>, env: HashMap<String, String>) -> Self {
        Self { spec, dir, port, distro, env, process: Mutex::new(None), starting: Mutex::new(()) }
    }

    pub fn log_path() -> PathBuf {
        state_dir().join("bot.log")
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    /// The bot's audio WebSocket, for the bridge.
    pub fn socket_url(&self) -> String {
        format!("ws://127.0.0.1:{}{}", self.port, self.spec.socket)
    }

    /// The token, read when needed: `make setup` may have just written it.
    pub fn token(&self) -> Option<String> {
        read_token(&self.dir)
    }

    /// `make setup`, when the folder's setup marker is missing.
    pub async fn setup_if_needed(&self) -> Result<bool, String> {
        let done = self.spec.setup_done.as_deref().is_some_and(|f| self.dir.join(f).exists());
        if done {
            return Ok(false);
        }
        run_on(self.distro.as_deref(), "make", &["setup"], &self.dir, &self.env, Duration::from_secs(20 * 60))
            .await
            .map_err(|e| format!("The bot's make setup failed: {e}"))?;
        Ok(true)
    }

    pub async fn health(&self) -> Health {
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(2)).build() {
            Ok(c) => c,
            Err(_) => return Health { up: false, ready: false },
        };
        match client.get(self.url(&self.spec.health)).send().await {
            Ok(r) => Health { up: true, ready: r.status().is_success() },
            Err(_) => Health { up: false, ready: false },
        }
    }

    /// Start `make dev PORT=<port>` and wait until the health address
    /// answers. `progress` gets a line every few seconds while it waits.
    pub async fn start(&self, progress: &impl Fn(&str)) -> Result<Health, String> {
        let _one = self.starting.lock().await;
        self.stop().await;
        std::fs::create_dir_all(state_dir()).map_err(|e| e.to_string())?;
        let log = std::fs::File::create(Self::log_path()).map_err(|e| e.to_string())?;
        let log_err = log.try_clone().map_err(|e| e.to_string())?;
        let script = format!("make dev PORT={}", self.port);
        let distro = self.distro.as_deref();
        let mut command = match distro {
            Some(_) => create_shell_command(&format!("exec bash -lc {}", sh_quote(&script)), wsl_env(&self.env),
                &command_path(&self.dir, distro), distro),
            None => create_shell_command(&script, self.env.clone(), &self.dir.to_string_lossy(), None),
        };
        command.stdin(Stdio::null()).stdout(Stdio::from(log)).stderr(Stdio::from(log_err));
        let child = spawn_command(command).map_err(|e| format!("could not start the bot's make dev: {e}"))?;
        *self.process.lock().await = Some(child);

        let deadline = tokio::time::Instant::now() + Duration::from_secs(self.spec.start_timeout);
        let mut next_update = tokio::time::Instant::now();
        loop {
            let health = self.health().await;
            if health.up {
                return Ok(health);
            }
            if let Some(child) = self.process.lock().await.as_mut() {
                if let Some(status) = child.child.try_wait().map_err(|e| e.to_string())? {
                    return Err(format!("The bot's make dev stopped ({status}):\n{}\n\nSee {}",
                        log_tail(12), Self::log_path().display()));
                }
            }
            if tokio::time::Instant::now() > deadline {
                return Err(format!("The bot did not answer at {} within {}s; see {}",
                    self.url(&self.spec.health), self.spec.start_timeout, Self::log_path().display()));
            }
            if tokio::time::Instant::now() >= next_update {
                progress(&format!("Waiting for the bot at {} (the first start compiles it). Details: {}",
                    self.url(&self.spec.health), Self::log_path().display()));
                next_update = tokio::time::Instant::now() + Duration::from_secs(10);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn stop(&self) {
        // Dropping the process kills its whole tree.
        self.process.lock().await.take();
    }
}

/// The page's bot routes, behind Builder's guard (secret, Host, Origin):
/// status, restart, the log, and the audio WebSocket bridge.
pub fn routes(bot: Arc<Bot>) -> Router {
    Router::new()
        .route("/bot/status", get(status))
        .route("/bot/restart", post(restart))
        .route("/bot/log", get(log))
        .route("/bot/socket", get(socket))
        .with_state(bot)
}

async fn status(State(bot): State<Arc<Bot>>) -> Json<serde_json::Value> {
    let health = bot.health().await;
    Json(json!({"name": bot.spec.name, "port": bot.port, "up": health.up, "ready": health.ready}))
}

/// After the agent changes the bot: a Rust program does not reload itself.
async fn restart(State(bot): State<Arc<Bot>>) -> Response {
    match bot.start(&|_| {}).await {
        Ok(health) => Json(json!({"up": health.up, "ready": health.ready})).into_response(),
        Err(error) => (StatusCode::BAD_GATEWAY, Json(json!({"error": error}))).into_response(),
    }
}

async fn log() -> Response {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], log_tail(200)).into_response()
}

/// The page talks to the bot as the switch would. A browser cannot send the
/// Authorization header the bot asks for, so the bridge adds it.
async fn socket(State(bot): State<Arc<Bot>>, ws: WebSocketUpgrade) -> Response {
    let Some(token) = bot.token() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "The bot has no EARSHOT_BOT_TOKEN in its .env; run its make setup.").into_response();
    };
    let url = bot.socket_url();
    ws.max_message_size(64 * 1024).on_upgrade(move |client| bridge(client, url, token))
}

async fn bridge(client: WebSocket, url: String, token: String) {
    let request = url.as_str().into_client_request().and_then(|mut request| {
        let value = format!("Bearer {token}").parse().map_err(|_| bot_ws::tungstenite::Error::Url(
            bot_ws::tungstenite::error::UrlError::NoHostName))?;
        request.headers_mut().insert(header::AUTHORIZATION, value);
        Ok(request)
    });
    let upstream = match request {
        Ok(request) => bot_ws::connect_async(request).await,
        Err(error) => Err(error),
    };
    let (mut client_tx, mut client_rx) = client.split();
    let upstream = match upstream {
        Ok((upstream, _)) => upstream,
        Err(error) => {
            warn!("Could not reach the bot at {url}: {error}");
            let _ = client_tx.send(Message::Text(json!({"type": "builder_error", "message": format!("The bot did not accept the call: {error}")}).to_string().into())).await;
            return;
        }
    };
    let (mut up_tx, mut up_rx) = upstream.split();
    let to_bot = async {
        while let Some(Ok(message)) = client_rx.next().await {
            let message = match message {
                Message::Text(text) => UpMessage::Text(text.as_str().into()),
                Message::Binary(bytes) => UpMessage::Binary(bytes),
                Message::Ping(bytes) => UpMessage::Ping(bytes),
                Message::Pong(bytes) => UpMessage::Pong(bytes),
                Message::Close(_) => break,
            };
            if up_tx.send(message).await.is_err() {
                break;
            }
        }
        let _ = up_tx.close().await;
    };
    let to_page = async {
        while let Some(Ok(message)) = up_rx.next().await {
            let message = match message {
                UpMessage::Text(text) => Message::Text(text.as_str().into()),
                UpMessage::Binary(bytes) => Message::Binary(bytes),
                UpMessage::Ping(bytes) => Message::Ping(bytes),
                UpMessage::Pong(bytes) => Message::Pong(bytes),
                UpMessage::Close(_) => break,
                UpMessage::Frame(_) => continue,
            };
            if client_tx.send(message).await.is_err() {
                break;
            }
        }
        let _ = client_tx.close().await;
    };
    tokio::select! { _ = to_bot => {}, _ = to_page => {} }
}

/// The last `n` lines of the bot's log.
pub fn log_tail(n: usize) -> String {
    let log = std::fs::read_to_string(Bot::log_path()).unwrap_or_default();
    let lines: Vec<&str> = log.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_bot_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_spec(dir.path()).is_err());
        std::fs::write(dir.path().join("builder.toml"), "[builder]\nkind = \"app\"\nname = \"x\"\n").unwrap();
        assert!(read_spec(dir.path()).unwrap_err().contains("kind = \"bot\""));
        std::fs::write(dir.path().join("builder.toml"),
            "[builder]\nkind = \"bot\"\nname = \"Noa\"\nport = 8091\nsetup_done = \".env\"\n").unwrap();
        let spec = read_spec(dir.path()).unwrap();
        assert_eq!((spec.name.as_str(), spec.port, spec.health.as_str(), spec.socket.as_str()), ("Noa", 8091, "/healthz", "/earshot"));
        assert_eq!(read_token(dir.path()), None);
        std::fs::write(dir.path().join(".env"), "OPENAI_API_KEY=k\nEARSHOT_BOT_TOKEN=\"abc\"\n").unwrap();
        assert_eq!(read_token(dir.path()).as_deref(), Some("abc"));
    }

    /// A stand-in bot: echoes binary frames, answers text with "ok", and
    /// refuses a connection without `Bearer <token>`.
    async fn echo_bot(token: &'static str) -> u16 {
        use axum::{extract::ws::WebSocketUpgrade, http::HeaderMap};
        let app = Router::new().route("/earshot", get(move |headers: HeaderMap, ws: WebSocketUpgrade| async move {
            if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some(&format!("Bearer {token}")) {
                return StatusCode::UNAUTHORIZED.into_response();
            }
            ws.on_upgrade(|mut socket| async move {
                while let Some(Ok(message)) = socket.recv().await {
                    let reply = match message {
                        Message::Binary(b) => Message::Binary(b),
                        Message::Text(_) => Message::Text("ok".into()),
                        _ => continue,
                    };
                    if socket.send(reply).await.is_err() { break; }
                }
            })
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        port
    }

    #[tokio::test]
    async fn the_bridge_adds_the_token_and_relays_audio() {
        let bot_port = echo_bot("t0ken").await;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), "EARSHOT_BOT_TOKEN=t0ken\n").unwrap();
        let spec = BotSpec { kind: "bot".into(), name: "t".into(), port: bot_port, health: "/healthz".into(),
            socket: "/earshot".into(), setup_done: None, start_timeout: 5 };
        let bot = Arc::new(Bot::new(spec, dir.path().to_path_buf(), bot_port, None, HashMap::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, routes(bot)).await.unwrap() });

        let (mut page, _) = bot_ws::connect_async(format!("ws://127.0.0.1:{port}/bot/socket")).await.unwrap();
        page.send(UpMessage::Binary(vec![1u8, 2, 3, 4].into())).await.unwrap();
        assert_eq!(page.next().await.unwrap().unwrap(), UpMessage::Binary(vec![1u8, 2, 3, 4].into()));
        page.send(UpMessage::Text(r#"{"type":"speech_started"}"#.into())).await.unwrap();
        assert_eq!(page.next().await.unwrap().unwrap(), UpMessage::Text("ok".into()));

        // A wrong token in .env: the bot refuses, and the page is told why.
        std::fs::write(dir.path().join(".env"), "EARSHOT_BOT_TOKEN=wrong\n").unwrap();
        let (mut page, _) = bot_ws::connect_async(format!("ws://127.0.0.1:{port}/bot/socket")).await.unwrap();
        let UpMessage::Text(text) = page.next().await.unwrap().unwrap() else { panic!("expected the error event") };
        assert!(text.contains("builder_error"), "{text}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn starts_waits_for_health_and_stops() {
        let dir = tempfile::tempdir().unwrap();
        let port = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        // A stand-in `make dev`: a tiny HTTP server answering 503 (no key yet).
        std::fs::write(dir.path().join("server.py"), "import http.server as h, sys\n\
class H(h.BaseHTTPRequestHandler):\n    def do_GET(s): s.send_response(503); s.end_headers()\n\
h.HTTPServer(('127.0.0.1', int(sys.argv[1])), H).serve_forever()\n").unwrap();
        std::fs::write(dir.path().join("Makefile"), "dev:\n\texec python3 server.py $(PORT)\n").unwrap();
        let spec = BotSpec { kind: "bot".into(), name: "t".into(), port, health: "/healthz".into(),
            socket: "/earshot".into(), setup_done: None, start_timeout: 20 };
        let env: HashMap<String, String> = std::env::vars().collect();
        let bot = Bot::new(spec, dir.path().to_path_buf(), port, None, env);
        let health = bot.start(&|_| {}).await.unwrap();
        assert_eq!(health, Health { up: true, ready: false });
        bot.stop().await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!bot.health().await.up);
    }
}
