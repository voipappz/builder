//! A voice bot or product beside the app: a folder whose `builder.toml`
//! says `kind = "bot"` or `kind = "product"` (Connectix, the bot brain, is
//! one). Builder runs the folder's own `make setup` and `make dev`,
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
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

use crate::command::{create_shell_command, spawn_command, ChildProcess};
use crate::launcher::{command_path, run_on, sh_quote, state_dir, wsl_env};

/// What a folder is: a voice bot, or a product (Connectix, the bot brain).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Bot,
    Product,
}

impl Kind {
    /// The word for it in messages: "bot" or "product".
    pub fn noun(self) -> &'static str {
        match self { Kind::Bot => "bot", Kind::Product => "product" }
    }
}

/// The `[builder]` table of a bot folder's `builder.toml`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BotSpec {
    pub kind: Kind,
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
    /// A make target that stops what `make dev` leaves running (a detached
    /// container, say), run after the process is killed.
    #[serde(default)]
    pub stop: Option<String>,
    /// Path of the folder's VoipAppz dev MCP (e.g. `/voipappz/mcp`); the agent
    /// gets it beside the API's.
    #[serde(default)]
    pub mcp: Option<String>,
    /// Path of the page shown in Builder's Product pane.
    #[serde(default)]
    pub page: Option<String>,
    /// Commands that must work before setup, e.g. `["docker"]`.
    #[serde(default)]
    pub requires: Vec<String>,
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
        .map_err(|e| format!("{}: {e}. A bot folder has a builder.toml with kind = \"bot\" or \"product\".", path.display()))?;
    let spec = toml::from_str::<BuilderToml>(&text)
        .map_err(|e| format!("{}: {e}. A bot folder says kind = \"bot\" or \"product\".", path.display()))?.builder;
    let paths = [Some(&spec.health), Some(&spec.socket), spec.mcp.as_ref(), spec.page.as_ref()];
    if paths.into_iter().flatten().any(|p| !p.starts_with('/')) {
        return Err(format!("{}: health, socket, mcp and page are paths starting with /", path.display()));
    }
    // A leading '-' would read as an option to make or `command -v`.
    let name = |t: &String, extra: &str| t.starts_with(|c: char| c.is_ascii_alphanumeric())
        && t.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c));
    if spec.stop.as_ref().is_some_and(|t| !name(t, "-_")) || !spec.requires.iter().all(|t| name(t, "-_.")) {
        return Err(format!("{}: stop names a make target and requires names commands (letters, digits, - _ and . in commands)", path.display()));
    }
    Ok(spec)
}

/// The shared token the bot asks of Earshot, from the bot's `.env`, read as
/// `make dev` sources it: `export` allowed, comments skipped, the last
/// assignment wins.
pub fn read_token(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(".env")).ok()?;
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let line = line.strip_prefix("export ").map(str::trim_start).unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            (key == "EARSHOT_BOT_TOKEN").then(|| env_value(value))
        })
        .next_back()
        .filter(|v| !v.is_empty())
}

/// A `.env` value as sh reads it: quoted up to the closing quote, otherwise
/// up to a ` #` comment.
fn env_value(value: &str) -> String {
    let value = value.trim();
    for quote in ['"', '\''] {
        if let Some(rest) = value.strip_prefix(quote) {
            return rest.split(quote).next().unwrap_or_default().to_string();
        }
    }
    value.split(" #").next().unwrap_or_default().trim().to_string()
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
    /// Set when Builder stops: a restart still waiting does not start it again.
    stopped: AtomicBool,
    /// Set after the first start: only that one clears what a crashed run left.
    started: AtomicBool,
}

impl std::fmt::Debug for Bot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bot").field("name", &self.spec.name).field("dir", &self.dir).field("port", &self.port).finish()
    }
}

impl Bot {
    pub fn new(spec: BotSpec, dir: PathBuf, port: u16, distro: Option<String>, env: HashMap<String, String>) -> Self {
        Self { spec, dir, port, distro, env, process: Mutex::new(None), starting: Mutex::new(()),
            stopped: AtomicBool::new(false), started: AtomicBool::new(false) }
    }

    /// One log per port, so two Builders' bots do not share one.
    pub fn log_path(&self) -> PathBuf {
        state_dir().join(format!("bot-{}.log", self.port))
    }

    /// Where it answers: `http://127.0.0.1:<port>`.
    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin())
    }

    /// The folder's dev MCP server, when its builder.toml names one.
    pub fn mcp_url(&self) -> Option<String> {
        self.spec.mcp.as_deref().map(|path| self.url(path))
    }

    /// The page for Builder's Product pane, when its builder.toml names one.
    pub fn page_url(&self) -> Option<String> {
        self.spec.page.as_deref().map(|path| self.url(path))
    }

    /// The commands its builder.toml requires, found in the folder's system.
    pub async fn check_requires(&self) -> Result<(), String> {
        let native_windows = self.distro.is_none() && cfg!(windows);
        for tool in &self.spec.requires {
            let check = if tool == "docker" { "docker info --format '{{.ServerVersion}}'".to_string() }
                else { format!("command -v {}", sh_quote(tool)) };
            let result = match (native_windows, tool.as_str()) {
                (true, "docker") => run_on(None, "docker", &["info", "--format", "{{.ServerVersion}}"], &self.dir, &self.env, Duration::from_secs(30)).await,
                (true, _) => run_on(None, "where", &[tool], &self.dir, &self.env, Duration::from_secs(30)).await,
                (false, _) => run_on(self.distro.as_deref(), "sh", &["-c", &check], &self.dir, &self.env, Duration::from_secs(30)).await,
            };
            result.map_err(|e| format!("{} needs {tool}, which did not answer: {e}", self.spec.name))?;
        }
        Ok(())
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
            .map_err(|e| format!("The {}'s make setup failed: {e}", self.spec.kind.noun()))?;
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
        if self.stopped.load(Ordering::SeqCst) {
            return Err("Builder is stopping.".into());
        }
        self.process.lock().await.take();
        // The first start clears what a crashed run on this port left behind (a
        // detached container); restarts after an edit keep it, `make dev`
        // brings it up to date.
        if !self.started.swap(true, Ordering::SeqCst) {
            self.run_stop().await;
        }
        std::fs::create_dir_all(state_dir()).map_err(|e| e.to_string())?;
        let log = std::fs::File::create(self.log_path()).map_err(|e| e.to_string())?;
        let log_err = log.try_clone().map_err(|e| e.to_string())?;
        let script = format!("make dev PORT={}", self.port);
        let distro = self.distro.as_deref();
        let mut command = match distro {
            Some(_) => create_shell_command(&format!("exec bash -lc {}", sh_quote(&script)), wsl_env(&self.env),
                &command_path(&self.dir, distro), distro),
            None => {
                // Only what Builder chose to pass: none of its own logins or keys.
                let mut command = create_shell_command(&script, HashMap::new(), &self.dir.to_string_lossy(), None);
                command.env_clear().envs(&self.env);
                command
            }
        };
        command.stdin(Stdio::null()).stdout(Stdio::from(log)).stderr(Stdio::from(log_err));
        let child = spawn_command(command).map_err(|e| format!("could not start the {}'s make dev: {e}", self.spec.kind.noun()))?;
        *self.process.lock().await = Some(child);
        // stop() may have run between the check above and now.
        if self.stopped.load(Ordering::SeqCst) {
            self.process.lock().await.take();
            return Err("Builder is stopping.".into());
        }

        let deadline = tokio::time::Instant::now() + Duration::from_secs(self.spec.start_timeout);
        let mut next_update = tokio::time::Instant::now();
        loop {
            if self.stopped.load(Ordering::SeqCst) {
                return Err("Builder is stopping.".into());
            }
            let health = self.health().await;
            if health.up {
                return Ok(health);
            }
            let mut process = self.process.lock().await;
            if let Some(status) = match process.as_mut() { Some(child) => child.child.try_wait().map_err(|e| e.to_string())?, None => None } {
                // A `make dev` that only starts a detached container returns at
                // once; its folder names a stop target, and the wait goes on.
                if !(status.success() && self.spec.stop.is_some()) {
                    return Err(format!("The {}'s make dev stopped ({status}):\n{}\n\nSee {}",
                        self.spec.kind.noun(), self.log_tail(12), self.log_path().display()));
                }
                process.take();
            }
            drop(process);
            if tokio::time::Instant::now() > deadline {
                return Err(format!("The {} did not answer at {} within {}s; see {}", self.spec.kind.noun(),
                    self.url(&self.spec.health), self.spec.start_timeout, self.log_path().display()));
            }
            if tokio::time::Instant::now() >= next_update {
                progress(&format!("Waiting for the {} at {} (the first start builds it). Details: {}",
                    self.spec.kind.noun(), self.url(&self.spec.health), self.log_path().display()));
                next_update = tokio::time::Instant::now() + Duration::from_secs(10);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// For good, when Builder stops. Dropping the process kills its whole tree.
    pub async fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.process.lock().await.take();
        // A start in flight sees `stopped` within half a second and returns;
        // waiting for it means its `make dev` is not spawned after the stop
        // target ran.
        let _one = self.starting.lock().await;
        self.process.lock().await.take();
        self.run_stop().await;
    }

    /// The folder's stop target with the same PORT `make dev` got, if it has
    /// one; bounded, so quitting Builder never hangs on it, and a failure is
    /// only logged.
    async fn run_stop(&self) {
        let Some(target) = &self.spec.stop else { return };
        let port = format!("PORT={}", self.port);
        if let Err(e) = run_on(self.distro.as_deref(), "make", &[target, &port], &self.dir, &self.env, Duration::from_secs(30)).await {
            warn!("{}: make {target} failed: {e}", self.spec.name);
        }
    }

    /// The last `n` lines of the bot's log.
    pub fn log_tail(&self, n: usize) -> String {
        let log = std::fs::read_to_string(self.log_path()).unwrap_or_default();
        let lines: Vec<&str> = log.lines().collect();
        lines[lines.len().saturating_sub(n)..].join("\n")
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

async fn log(State(bot): State<Arc<Bot>>) -> Response {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], bot.log_tail(200)).into_response()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_bot_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_spec(dir.path()).is_err());
        std::fs::write(dir.path().join("builder.toml"), "[builder]\nkind = \"app\"\nname = \"x\"\n").unwrap();
        assert!(read_spec(dir.path()).unwrap_err().contains("kind = \"bot\" or \"product\""));
        std::fs::write(dir.path().join("builder.toml"),
            "[builder]\nkind = \"bot\"\nname = \"Noa\"\nport = 8091\nsetup_done = \".env\"\n").unwrap();
        let spec = read_spec(dir.path()).unwrap();
        assert_eq!((spec.name.as_str(), spec.port, spec.health.as_str(), spec.socket.as_str()), ("Noa", 8091, "/healthz", "/earshot"));
        assert_eq!(read_token(dir.path()), None);
        std::fs::write(dir.path().join("builder.toml"),
            "[builder]\nkind = \"product\"\nname = \"Connectix\"\nmcp = \"/voipappz/mcp\"\nstop = \"down\"\nrequires = [\"docker\"]\n").unwrap();
        let product = read_spec(dir.path()).unwrap();
        assert_eq!((product.kind, product.stop.as_deref()), (Kind::Product, Some("down")));
        std::fs::write(dir.path().join("builder.toml"), "[builder]\nkind = \"product\"\nname = \"x\"\nstop = \"down; rm -rf /\"\n").unwrap();
        assert!(read_spec(dir.path()).is_err());
        std::fs::write(dir.path().join("builder.toml"), "[builder]\nkind = \"product\"\nname = \"x\"\nstop = \"-n\"\n").unwrap();
        assert!(read_spec(dir.path()).is_err(), "a leading - reads as an option to make");
        std::fs::write(dir.path().join("builder.toml"), "[builder]\nkind = \"product\"\nname = \"x\"\nrequires = [\"python3.12\"]\n").unwrap();
        assert!(read_spec(dir.path()).is_ok());
        std::fs::write(dir.path().join("builder.toml"), "[builder]\nkind = \"product\"\nname = \"x\"\nmcp = \"voipappz\"\n").unwrap();
        assert!(read_spec(dir.path()).is_err());
        std::fs::write(dir.path().join("builder.toml"),
            "[builder]\nkind = \"bot\"\nname = \"Noa\"\nport = 8091\nsetup_done = \".env\"\n").unwrap();
        std::fs::write(dir.path().join(".env"), "OPENAI_API_KEY=k\nEARSHOT_BOT_TOKEN=\"abc\"\n").unwrap();
        assert_eq!(read_token(dir.path()).as_deref(), Some("abc"));
        // As sh sources it: export, comments, the last assignment wins.
        std::fs::write(dir.path().join(".env"),
            "EARSHOT_BOT_TOKEN=old\n# EARSHOT_BOT_TOKEN=commented\nexport EARSHOT_BOT_TOKEN=new # a note\n").unwrap();
        assert_eq!(read_token(dir.path()).as_deref(), Some("new"));
        std::fs::write(dir.path().join(".env"), "EARSHOT_BOT_TOKEN='a b' # q\n").unwrap();
        assert_eq!(read_token(dir.path()).as_deref(), Some("a b"));
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
        let spec = BotSpec { kind: Kind::Bot, name: "t".into(), port: bot_port, health: "/healthz".into(),
            socket: "/earshot".into(), setup_done: None, start_timeout: 5, stop: None, mcp: None, page: None,
            requires: vec![] };
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
        // `stop` stands for what a detached container would need: it leaves a mark.
        std::fs::write(dir.path().join("Makefile"),
            "dev:\n\texec python3 server.py $(PORT)\nhalt:\n\ttouch stopped\n").unwrap();
        let spec = BotSpec { kind: Kind::Product, name: "t".into(), port, health: "/healthz".into(),
            socket: "/earshot".into(), setup_done: None, start_timeout: 20, stop: Some("halt".into()),
            mcp: Some("/voipappz/mcp".into()), page: Some("/admin".into()), requires: vec!["python3".into()] };
        let env: HashMap<String, String> = std::env::vars().collect();
        let bot = Bot::new(spec, dir.path().to_path_buf(), port, None, env);
        bot.check_requires().await.unwrap();
        assert_eq!(bot.mcp_url(), Some(format!("http://127.0.0.1:{port}/voipappz/mcp")));
        assert_eq!(bot.page_url(), Some(format!("http://127.0.0.1:{port}/admin")));
        let health = bot.start(&|_| {}).await.unwrap();
        assert_eq!(health, Health { up: true, ready: false });
        std::fs::remove_file(dir.path().join("stopped")).unwrap();
        bot.stop().await;
        assert!(dir.path().join("stopped").exists(), "stop runs the folder's stop target");
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!bot.health().await.up);
        // A restart that comes after Builder stopped does not start it again.
        assert!(bot.start(&|_| {}).await.unwrap_err().contains("stopping"));
        assert!(!bot.health().await.up);
    }

    /// A `make dev` that only starts something detached (`docker compose up
    /// -d`) and returns: with a stop target, Builder keeps waiting for health,
    /// and stop brings it down with the same PORT.
    #[tokio::test]
    async fn a_detached_make_dev_is_waited_for_and_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let port = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        std::fs::write(dir.path().join("server.py"), "import http.server as h, sys, time\n\
time.sleep(1)\n\
class H(h.BaseHTTPRequestHandler):\n    def do_GET(s): s.send_response(200); s.end_headers()\n\
h.HTTPServer(('127.0.0.1', int(sys.argv[1])), H).serve_forever()\n").unwrap();
        std::fs::write(dir.path().join("Makefile"),
            "dev:\n\tsetsid python3 server.py $(PORT) >/dev/null 2>&1 & echo $$! > pid.$(PORT)\n\
down:\n\t-kill $$(cat pid.$(PORT)) && rm pid.$(PORT)\n").unwrap();
        let spec = BotSpec { kind: Kind::Product, name: "t".into(), port, health: "/".into(),
            socket: "/earshot".into(), setup_done: None, start_timeout: 20, stop: Some("down".into()),
            mcp: None, page: None, requires: vec![] };
        let env: HashMap<String, String> = std::env::vars().collect();
        let bot = Bot::new(spec, dir.path().to_path_buf(), port, None, env);
        assert_eq!(bot.start(&|_| {}).await.unwrap(), Health { up: true, ready: true });
        bot.stop().await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!bot.health().await.up, "stop ran `make down PORT=<port>`");
        assert!(!dir.path().join(format!("pid.{port}")).exists());
    }
}
