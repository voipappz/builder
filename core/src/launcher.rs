//! Starting VoipAppz Builder from saved settings: the settings file, the
//! app's dev server, the API login check and the local server. The desktop
//! app uses this; the CLI shares the API check.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{info, warn};

use crate::builder::{self, ApiMcp, BuilderConfig};
use crate::command::{create_shell_command, spawn_command, ChildProcess};

pub const DEFAULT_AGENT_COMMAND: &str = "npx -y @agentclientprotocol/claude-agent-acp@0.94.0";
/// The agent from the app's own packages, on the app's pinned Node.
pub const APP_AGENT_COMMAND: &str = "bin/npm.sh exec -- claude-agent-acp";

/// The configured agent, except that the default `npx` one gives way to the
/// app's own copy once the working copy's packages include it.
pub fn agent_command(configured: &str, workspace: &Path) -> String {
    let app_has_agent = workspace.join("bin/npm.sh").is_file()
        && workspace.join("node_modules/.bin/claude-agent-acp").exists();
    if configured == DEFAULT_AGENT_COMMAND && app_has_agent { APP_AGENT_COMMAND } else { configured }.to_string()
}

/// Setup steps, in order, as `(id, label)`. `start` reports entering a step
/// with a progress line of `STEP_PREFIX` followed by the id, so a UI can
/// show a checklist; other lines are details for the current step.
///
/// Builder runs no Node itself: the app's own Makefile installs its tools
/// and packages (`make setup`) and starts it (`make dev`).
pub const SETUP_STEPS: &[(&str, &str)] = &[
    ("check", "Pre-flight checks"),
    ("workspace", "App working copy"),
    ("setup", "App setup (make setup)"),
    ("app", "App dev server (make dev)"),
    ("bot", "Voice bot (make dev)"),
    ("api", "API connection"),
    ("server", "Builder connection"),
];
pub const STEP_PREFIX: &str = "@step ";

pub fn step_label(id: &str) -> Option<&'static str> {
    SETUP_STEPS.iter().find(|(step, _)| *step == id).map(|(_, label)| *label)
}

/// Disk space setup needs: packages, the Playwright browser and the agent.
const MIN_FREE_BYTES: u64 = 3 * 1024 * 1024 * 1024;

/// The API login typed in onboarding. The app's login gate (`make setup`,
/// `make dev`) reads it as TEST_EMAIL and TEST_PASSWORD; the agent never
/// sees it.
#[derive(Clone)]
pub struct ApiLogin {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level { Ok, Warn, Fail }

/// One pre-flight result: shown before setup runs; a `Fail` stops setup.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub label: String,
    pub level: Level,
    pub detail: String,
}

impl Check {
    fn new(label: &str, level: Level, detail: impl Into<String>) -> Self {
        Self { label: label.into(), level, detail: detail.into() }
    }
    pub fn line(&self) -> String {
        let mark = match self.level { Level::Ok => "✓", Level::Warn => "⚠", Level::Fail => "✕" };
        format!("{mark} {}: {}", self.label, self.detail)
    }
}

/// The WSL distro an app folder lives in, if it is under `\\wsl.localhost`.
/// On Windows, Builder then runs git, the app's Makefile and the agent there.
pub fn wsl_distro(settings: &Settings) -> Option<String> {
    let repo = settings.app_repo.as_deref().or(settings.workspace.as_deref())?;
    crate::wsl::location(&repo.to_string_lossy()).map(|(distro, _)| distro)
}

/// `path` as the commands see it: the Linux path inside the distro, or as is.
pub(crate) fn command_path(path: &Path, distro: Option<&str>) -> String {
    match (distro, crate::wsl::location(&path.to_string_lossy())) {
        (Some(_), Some((_, linux))) => linux,
        _ => path.to_string_lossy().into_owned(),
    }
}

/// Single-quote `s` for sh.
pub(crate) fn sh_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./=:@%+,".contains(c)) {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// What a command inside the distro gets from Builder: its PATH and the
/// variables Builder adds. wsl.exe gets them through WSLENV (PATH on the
/// command line), and the distro gives HOME, USER and the rest itself.
pub(crate) fn wsl_env(env: &HashMap<String, String>) -> HashMap<String, String> {
    env.iter()
        .filter(|(k, _)| matches!(k.as_str(), "PATH" | "TEST_EMAIL" | "TEST_PASSWORD" | "VA_TEST_OTP" | "VITE_API_BASE_URL")
            || k.starts_with("ANTHROPIC_") || k.starts_with("CLAUDE_"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Whether a distro's `uname -r` is a WSL2 kernel ("…-microsoft-standard-WSL2");
/// WSL1 reports a version ending in "-Microsoft".
pub fn wsl2_kernel(release: &str) -> bool {
    let release = release.trim().to_ascii_lowercase();
    release.contains("wsl2") || (release.contains("microsoft-standard") && !release.ends_with("-microsoft"))
}

/// Free bytes on the volume holding `path`.
pub fn free_bytes(path: &Path) -> Option<u64> {
    let path = path.ancestors().find(|p| p.exists())?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
        let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
        (unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0).then(|| st.f_bavail as u64 * st.f_frsize as u64)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free: winapi::um::winnt::ULARGE_INTEGER = unsafe { std::mem::zeroed() };
        let ok = unsafe { winapi::um::fileapi::GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) };
        (ok != 0).then(|| unsafe { *free.QuadPart() })
    }
}

/// What can be known before setup starts. Nothing here runs Node.
pub async fn preflight(settings: &Settings, env: &HashMap<String, String>) -> Vec<Check> {
    let mut checks = Vec::new();
    let path = env.iter().find(|(k, _)| k.eq_ignore_ascii_case("PATH")).map(|(_, v)| v.clone());
    let cwd = std::env::temp_dir();
    let found = |tool: &str| which::which_in(tool, path.as_ref(), &cwd).is_ok();

    let repo = settings.app_repo.as_deref().or(settings.workspace.as_deref());
    let distro = wsl_distro(settings);
    if let Some(distro) = &distro {
        // Builder's own PATH: `env` is the distro's environment here.
        if cfg!(windows) && which::which("wsl.exe").is_err() {
            checks.push(Check::new("WSL", Level::Fail, "wsl.exe not found; the app folder is in WSL"));
        } else {
            checks.push(Check::new("WSL", Level::Ok, format!("the app runs in the {distro} distro")));
        }
    }
    match repo {
        None => checks.push(Check::new("App folder", Level::Fail, "Choose the voipappz/app checkout.")),
        Some(repo) if distro.is_none() && (is_network_path(&repo.to_string_lossy()) || repo.to_string_lossy().starts_with(r"\\?\UNC")) =>
            checks.push(Check::new("App folder", Level::Fail, format!("{} is on a network share; choose a local or WSL checkout.", repo.display()))),
        Some(repo) if !repo.join("Makefile").is_file() || !repo.join("package.json").is_file() =>
            checks.push(Check::new("App folder", Level::Fail, format!("{} has no Makefile and package.json; choose the voipappz/app checkout.", repo.display()))),
        Some(repo) => checks.push(Check::new("App folder", Level::Ok, repo.display().to_string())),
    }

    // Inside WSL the tools must be found in the distro, not on Windows.
    let in_distro: Option<Vec<String>> = match (&distro, repo) {
        (Some(d), Some(repo)) => run_on(Some(d), "sh", &["-c", "uname -r; for t in git make bash curl; do command -v $t >/dev/null && echo $t; done"],
            repo, env, Duration::from_secs(60)).await.ok().map(|out| out.lines().map(str::to_string).collect()),
        _ => None,
    };
    // The previews reach the dev server on localhost, which only WSL2
    // forwards to Windows; the distro's kernel names its WSL version.
    if let (Some(d), Some(lines)) = (&distro, &in_distro) {
        let kernel = lines.first().map(String::as_str).unwrap_or_default();
        checks.push(if wsl2_kernel(kernel) {
            Check::new("WSL version", Level::Ok, format!("WSL2 ({kernel})"))
        } else {
            Check::new("WSL version", Level::Fail, format!("{d} runs on WSL1 ({kernel}); Builder needs WSL2. In PowerShell: wsl --set-version {d} 2"))
        });
    }
    for (tool, why) in [("git", "Builder makes its working copy with git"), ("make", "Builder runs the app with its Makefile"), ("bash", "the app's Makefile needs bash"), ("curl", "the app's Makefile needs curl")] {
        let present = match (&distro, &in_distro) {
            (Some(_), Some(tools)) => tools.iter().any(|t| t == tool),
            (Some(_), None) => false,
            (None, _) => found(tool),
        };
        if present {
            checks.push(Check::new(tool, Level::Ok, "found"));
        } else {
            let hint = match &distro {
                Some(d) => format!(" Install it in {d}: sudo apt install {tool}"),
                None if cfg!(windows) => " On Windows, choose the app checkout inside WSL (\\\\wsl.localhost\\…); Builder runs it there.".to_string(),
                None => String::new(),
            };
            checks.push(Check::new(tool, Level::Fail, format!("not found; {why}.{hint}")));
        }
    }

    // `make setup` and `make dev` start with the app's login gate; without a
    // login and no TTY it stops. A login saved in .env is in `env` already
    // (add_saved_login).
    let setup_due = planned_workspace(settings).is_none_or(|w| !w.join(SETUP_DONE).is_file());
    if repo.is_some() && (settings.start_app || setup_due) {
        let set = |key: &str| env.get(key).is_some_and(|v| !v.is_empty());
        if set("TEST_EMAIL") && set("TEST_PASSWORD") {
            checks.push(Check::new("API login", Level::Ok, "set"));
        } else {
            checks.push(Check::new("API login", Level::Fail,
                "none; enter your API email and password in Builder's setup. The app's make setup and make dev sign in with them."));
        }
    }

    // A WSL distro's disk is a file on the Windows drive: both need room.
    let mut disks = vec![("Disk space", repo.map(Path::to_path_buf).unwrap_or_else(state_dir))];
    if distro.is_some() {
        disks.push(("Windows disk space", state_dir()));
    }
    for (label, disk) in disks {
        match free_bytes(&disk) {
            Some(free) if free < MIN_FREE_BYTES => checks.push(Check::new(label, Level::Fail,
                format!("{} MB free; setup needs about {} MB. Free some space and retry.", free / 1_048_576, MIN_FREE_BYTES / 1_048_576))),
            Some(free) => checks.push(Check::new(label, Level::Ok, format!("{} GB free", free / 1_073_741_824))),
            None => checks.push(Check::new(label, Level::Warn, "could not be measured")),
        }
    }

    let wanted = url_port(&settings.client_url).unwrap_or(3000);
    match free_port(wanted).await {
        Some(port) if port == wanted => checks.push(Check::new("App port", Level::Ok, format!("{port} is free"))),
        Some(port) => checks.push(Check::new("App port", Level::Warn, format!("{wanted} is taken (another app, or WSL); the app will use {port}"))),
        None => checks.push(Check::new("App port", Level::Fail, format!("no free port between {wanted} and {}", wanted + 19))),
    }
    match free_port(settings.port).await {
        Some(port) if port == settings.port => checks.push(Check::new("Builder port", Level::Ok, format!("{port} is free"))),
        Some(port) => checks.push(Check::new("Builder port", Level::Warn, format!("{} is taken (another Builder?); using {port}", settings.port))),
        None => checks.push(Check::new("Builder port", Level::Fail, format!("no free port from {}", settings.port))),
    }

    if let Some(api) = settings.api_url.as_deref() {
        let reachable = reqwest::Client::builder().timeout(Duration::from_secs(8)).build().ok();
        let answer = match reachable { Some(c) => c.get(api).send().await.is_ok(), None => false };
        checks.push(if answer { Check::new("API", Level::Ok, format!("{api} answers")) }
            else { Check::new("API", Level::Warn, format!("{api} does not answer; the previews will not sign in")) });
    }
    checks
}

const DEV_SERVER_WAIT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// The app repository (../app). Builder works in its own git worktree
    /// of it, created from origin/main on first start.
    pub app_repo: Option<PathBuf>,
    /// The checkout the agent works in. Defaults to `<app_repo>-builder`.
    pub workspace: Option<PathBuf>,
    /// Where the chat client loads from: the app's dev server or a deployment.
    pub client_url: String,
    /// The app shown in the previews; defaults to `client_url`.
    pub preview_url: Option<String>,
    /// VoipAppz API whose /api/mcp tools the agent gets. Defaults to
    /// http://127.0.0.1:<VA_WEB_PORT> from `api_dir`'s .env.
    pub api_url: Option<String>,
    /// The voipappz-api checkout. Its `bin/dev-status.sh` gives the
    /// /api/mcp login (as `make claude` does) when VA_MCP_BASIC is unset.
    pub api_dir: Option<PathBuf>,
    /// Start `npm run dev` in the workspace when the client does not answer.
    pub start_app: bool,
    pub port: u16,
    pub agent_command: String,
    /// A voice bot folder (`builder.toml` with `kind = "bot"`). Builder runs
    /// it beside the app, and the agent works in it instead of the app.
    pub bot_dir: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app_repo: None,
            workspace: None,
            client_url: "http://localhost:3000".to_string(),
            preview_url: None,
            api_url: None,
            api_dir: None,
            start_app: true,
            port: 9832,
            agent_command: DEFAULT_AGENT_COMMAND.to_string(),
            bot_dir: None,
        }
    }
}

impl Settings {
    pub fn dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("voipappz-builder")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("builder.toml")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        toml::from_str(&text).map_err(|e| format!("{}: {}", path.display(), e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        std::fs::create_dir_all(Self::dir()).map_err(|e| e.to_string())?;
        let body = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        let text = format!(
            "# VoipAppz Builder settings. Restart Builder after changing them.\n{}",
            body
        );
        std::fs::write(&path, text).map_err(|e| format!("{}: {}", path.display(), e))
    }
}

/// Folder for logs: ~/.local/state/voipappz-builder on Linux, the
/// platform's local data folder elsewhere.
pub fn state_dir() -> PathBuf {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("voipappz-builder")
}

/// An https origin, or plain http to localhost or 127.0.0.1.
pub fn valid_origin(url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url) else { return false; };
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    (url.scheme() == "https" || (url.scheme() == "http" && local))
        && url.host_str().is_some() && url.username().is_empty() && url.password().is_none()
        && url.path() == "/" && url.query().is_none() && url.fragment().is_none()
}

/// `preferred` if it can be bound on loopback, else the first free port of
/// the next twenty. On Windows with WSL in mirrored networking mode a port
/// can stay unusable after the Linux side closed it, with nothing listed.
async fn free_port(preferred: u16) -> Option<u16> {
    for port in preferred..preferred.saturating_add(20) {
        if tokio::net::TcpListener::bind(("127.0.0.1", port)).await.is_ok() {
            return Some(port);
        }
    }
    None
}

/// The port of `http://host[:port]`, if any.
pub fn url_port(url: &str) -> Option<u16> {
    reqwest::Url::parse(url).ok().and_then(|u| u.port())
}

/// `url` with its port set to `port`.
pub fn with_port(url: &str, port: u16) -> String {
    let Ok(mut parsed) = reqwest::Url::parse(url) else { return url.to_string() };
    if parsed.set_port(Some(port)).is_err() {
        return url.to_string();
    }
    parsed.to_string().trim_end_matches('/').to_string()
}

async fn answers(url: &str) -> bool {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(2)).build() {
        Ok(c) => c,
        Err(_) => return false,
    };
    client.get(url).send().await.is_ok()
}

/// The API's MCP server, after checking the API answers and accepts the
/// login. Builder offers it to the agent read-only. The error says why the
/// tools are off.
pub async fn api_mcp_server(api_url: &str, basic: Option<&str>) -> Result<ApiMcp, String> {
    let api_url = api_url.trim_end_matches('/');
    if !valid_origin(api_url) {
        return Err("Use an HTTPS API address, or HTTP on localhost for a local API.".into());
    }
    let basic = match basic {
        Some(b) if !b.is_empty() => b,
        _ => return Err("VA_MCP_BASIC is not set (in voipappz-api: make tenant, then MCP_AUTH=1 bin/dev-status.sh)".into()),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let listed = client
        .post(format!("{}/api/mcp", api_url))
        .header("Authorization", format!("Basic {}", basic))
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        .send()
        .await;
    match listed {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.map_err(|_| "The API returned an invalid MCP response.".to_string())?;
            if body.get("error").is_some() || !body.pointer("/result/tools").is_some_and(|v| v.is_array()) {
                return Err("The API did not return an MCP tools list.".into());
            }
        }
        Ok(r) => return Err(format!("{}/api/mcp answered {} for the VA_MCP_BASIC login", api_url, r.status())),
        Err(_) => return Err(format!("{}/api/mcp does not answer", api_url)),
    }
    Ok(ApiMcp {
        url: format!("{}/api/mcp", api_url),
        basic: basic.to_string(),
    })
}

/// Probe the actual API endpoint: /health can be routed to a separate service.
pub async fn check_api_connection(api_url: &str, email: &str, password: &str) -> Result<String, String> {
    let api_url = api_url.trim().trim_end_matches('/');
    if !valid_origin(api_url) {
        return Err("Use an HTTPS API address, or HTTP on localhost for a local API.".into());
    }
    if !email.is_empty() || !password.is_empty() {
        let basic = api_credential(email, password)?;
        api_mcp_server(api_url, Some(&basic)).await?;
        return Ok("Connected. Your API tools are available read-only. Sign in separately in the app previews.".into());
    }
    let response = reqwest::Client::builder().timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none()).build().map_err(|e| e.to_string())?
        .post(format!("{api_url}/api/mcp"))
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .send().await.map_err(|_| "Could not reach the API. Check the address and your internet connection.".to_string())?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED || response.status() == reqwest::StatusCode::FORBIDDEN {
        return Ok("API is reachable. Sign in in the app previews. Add API credentials below to enable read-only tools for the coding agent.".into());
    }
    if !response.status().is_success() {
        return Err(format!("The API endpoint returned {}. Check the address or try again later.", response.status()));
    }
    let body: serde_json::Value = response.json().await.map_err(|_| "This address did not return an MCP response.".to_string())?;
    if !body.pointer("/result/tools").is_some_and(|v| v.is_array()) {
        return Err("This address did not return an MCP tools list.".into());
    }
    Ok("API is reachable. Sign in in the app previews to use your account.".into())
}

pub fn api_credential(email: &str, password: &str) -> Result<String, String> {
    use base64::Engine;
    if email.trim().is_empty() || password.is_empty() || email.contains(':') {
        return Err("Enter both your API email and password.".into());
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(format!("{}:{password}", email.trim())))
}

/// A WSL (`\\wsl.localhost\…`, `\\wsl$\…`) or other network share path. On
/// Windows, `npm.cmd` runs through cmd.exe, which cannot use such a folder
/// as its working directory, so setup must refuse it up front.
pub fn is_network_path(path: &str) -> bool {
    let path = path.trim();
    path.starts_with("\\\\") || path.starts_with("//")
}

pub fn onboarding_settings(mut settings: Settings, app_repo: &str, api_url: &str, api_dir: &str) -> Result<Settings, String> {
    if is_network_path(app_repo) && crate::wsl::location(app_repo).is_none() {
        return Err("This folder is on a network share. Choose a checkout on a local drive or inside WSL (\\\\wsl.localhost\\<distro>\\…).".into());
    }
    let repo = PathBuf::from(app_repo.trim());
    if !repo.join("package.json").is_file() || !repo.join("src/builder-client/main.jsx").is_file() {
        return Err("Choose the React app folder containing package.json and src/builder-client/main.jsx.".into());
    }
    if !repo.join(".git").exists() {
        return Err("Choose a Git checkout of voipappz/app. Builder needs Git to create its separate working copy.".into());
    }
    let api_dir = (!api_dir.trim().is_empty()).then(|| PathBuf::from(api_dir.trim()));
    let url = onboarding_api_url(api_url, api_dir.as_deref())?;
    settings.app_repo = Some(if crate::wsl::location(app_repo).is_some() { repo } else { dunce::canonicalize(repo).map_err(|e| e.to_string())? });
    settings.workspace = None;
    settings.api_url = Some(url);
    settings.api_dir = api_dir;
    Ok(settings)
}

pub fn onboarding_api_url(api_url: &str, api_dir: Option<&Path>) -> Result<String, String> {
    let url = if let Some(dir) = api_dir {
        api_url_from_dir(dir).ok_or("The API checkout needs a .env containing VA_WEB_PORT. Start it with make dev first.")?
    } else {
        api_url.trim().trim_end_matches('/').to_owned()
    };
    if !valid_origin(&url) {
        return Err("Use an HTTPS API address, or HTTP on localhost for a local API.".into());
    }
    Ok(url)
}

/// A running Builder. Dropping it stops the dev server it started.
pub struct Running {
    pub url: String,
    pub workspace: PathBuf,
    pub api_note: String,
    shutdown: Option<oneshot::Sender<()>>,
    server: Option<tokio::task::JoinHandle<()>>,
    dev_server: Option<ChildProcess>,
    bot: Option<std::sync::Arc<crate::bot::Bot>>,
}

impl Running {
    /// Stop the server (and its agents) and the dev server it started.
    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(server) = self.server.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), server).await;
        }
        self.dev_server.take();
        if let Some(bot) = self.bot.take() {
            bot.stop().await;
        }
    }
}

/// The bot's `make setup` when needed, then `make dev` on a free port near
/// the one its builder.toml names. The bot gets no API login.
async fn start_bot(dir: &Path, env: &HashMap<String, String>, distro: Option<String>, progress: &impl Fn(&str))
    -> Result<std::sync::Arc<crate::bot::Bot>, String> {
    let spec = crate::bot::read_spec(dir)?;
    let port = free_port(spec.port).await
        .ok_or_else(|| format!("No free port between {} and {} for the bot.", spec.port, spec.port + 19))?;
    let env: HashMap<String, String> = env.iter()
        .filter(|(k, _)| !matches!(k.as_str(), "TEST_EMAIL" | "TEST_PASSWORD" | "VA_MCP_BASIC"))
        .map(|(k, v)| (k.clone(), v.clone())).collect();
    let bot = crate::bot::Bot::new(spec, dir.to_path_buf(), port, distro, env);
    progress(&format!("Voice bot: {} in {}", bot.spec.name, dir.display()));
    let setup = bot.setup_if_needed();
    if report_progress("Running the bot's setup (make setup)", progress, setup).await? {
        progress("The bot is set up.");
    }
    progress(&format!("Starting the bot (make dev PORT={port})…"));
    let health = bot.start(progress).await?;
    progress(&if health.ready {
        format!("The bot answers on port {port}.")
    } else {
        format!("The bot runs on port {port} but is not ready for calls: put OPENAI_API_KEY in {}.", dir.join(".env").display())
    });
    Ok(std::sync::Arc::new(bot))
}

fn start_dev_server(workspace: &Path, env: HashMap<String, String>, port: u16, distro: Option<&str>) -> Result<ChildProcess, String> {
    let log_dir = state_dir();
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let log = std::fs::File::create(log_dir.join("app.log")).map_err(|e| e.to_string())?;
    let log_err = log.try_clone().map_err(|e| e.to_string())?;
    // The app's own target: its login gate, then its dev server on `port`.
    let script = format!("make dev PORT={port}");
    let mut command = match distro {
        Some(_) => create_shell_command(&format!("exec bash -lc {}", sh_quote(&script)), wsl_env(&env), &command_path(workspace, distro), distro),
        None => create_shell_command(&script, env, &workspace.to_string_lossy(), None),
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    spawn_command(command).map_err(|e| format!("could not start make dev: {}", e))
}

/// The login saved in a checkout's .env (TEST_EMAIL, TEST_PASSWORD). A cached
/// .env.token is not enough: the app's login gate asks for both first.
pub fn app_login_in(dir: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(dir.join(".env")).ok()?;
    let value = |key: &str| text.lines().find_map(|line| {
        line.trim().strip_prefix(key).and_then(|rest| rest.strip_prefix('='))
            .map(|value| value.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
    }).filter(|value| !value.is_empty());
    Some((value("TEST_EMAIL")?, value("TEST_PASSWORD")?))
}

pub fn has_app_login(dir: &Path) -> bool {
    app_login_in(dir).is_some()
}

/// Unless a login was entered, the one saved in the workspace's or the app
/// repository's .env. Builder's new worktree has no .env of its own yet.
pub fn add_saved_login(settings: &Settings, env: &mut HashMap<String, String>) {
    if env.get("TEST_EMAIL").is_some_and(|e| !e.is_empty()) {
        return;
    }
    let dirs = planned_workspace(settings).into_iter().chain(settings.app_repo.clone());
    if let Some((email, password)) = dirs.into_iter().find_map(|dir| app_login_in(&dir)) {
        env.insert("TEST_EMAIL".into(), email);
        env.insert("TEST_PASSWORD".into(), password);
    }
}

/// The checkout Builder works in: the configured workspace, or its worktree.
fn planned_workspace(settings: &Settings) -> Option<PathBuf> {
    settings.workspace.clone().or_else(|| settings.app_repo.as_deref().map(default_worktree))
}

/// Present once `make setup` has installed the packages.
const SETUP_DONE: &str = "node_modules/.package-lock.json";

/// Builder's worktree next to the app repository: `../app` -> `../app-builder`.
pub fn default_worktree(app_repo: &Path) -> PathBuf {
    let name = app_repo
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "app".to_string());
    app_repo.with_file_name(format!("{}-builder", name))
}

/// `http://127.0.0.1:<VA_WEB_PORT>` from a voipappz-api checkout's .env.
pub fn api_url_from_dir(api_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(api_dir.join(".env")).ok()?;
    let port = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("VA_WEB_PORT="))
        .last()?
        .trim()
        .to_string();
    (!port.is_empty() && port.chars().all(|c| c.is_ascii_digit()))
        .then(|| format!("http://127.0.0.1:{}", port))
}

async fn run(program: &str, args: &[&str], cwd: &Path, env: &HashMap<String, String>) -> Result<String, String> {
    let limit = if args == ["--version"] { Duration::from_secs(20) }
        else if program == "git" { Duration::from_secs(120) }
        else { Duration::from_secs(15 * 60) };
    run_with_timeout(program, args, cwd, env, limit).await
}

/// Run `program` in `cwd` on this computer, or inside `distro` (the Linux
/// form of `cwd` and the distro's own login shell, so its tools are found).
pub(crate) async fn run_on(distro: Option<&str>, program: &str, args: &[&str], cwd: &Path, env: &HashMap<String, String>, limit: Duration) -> Result<String, String> {
    match distro {
        None => run_with_timeout(program, args, cwd, env, limit).await,
        Some(distro) => {
            let line = std::iter::once(program).chain(args.iter().copied()).map(sh_quote).collect::<Vec<_>>().join(" ");
            let command = create_shell_command(&format!("exec bash -lc {}", sh_quote(&line)), wsl_env(env), &command_path(cwd, Some(distro)), Some(distro));
            run_command(command, program, args, limit).await
        }
    }
}

async fn run_with_timeout(program: &str, args: &[&str], cwd: &Path, env: &HashMap<String, String>, limit: Duration) -> Result<String, String> {
    let mut command = tokio::process::Command::new(program);
    command.args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env);
    #[cfg(unix)]
    command.process_group(0);
    run_command(command, program, args, limit).await
}

async fn run_command(mut command: tokio::process::Command, program: &str, args: &[&str], limit: Duration) -> Result<String, String> {
    use tokio::io::AsyncReadExt;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut process = spawn_command(command)
        .map_err(|e| format!("{} could not run: {}", program, e))?;
    let mut stdout = process.child.stdout.take().ok_or("Command stdout unavailable")?;
    let mut stderr = process.child.stderr.take().ok_or("Command stderr unavailable")?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = tokio::time::timeout(limit, async {
        tokio::try_join!(process.child.wait(), stdout.read_to_end(&mut out), stderr.read_to_end(&mut err))
    }).await;
    let (status, _, _) = result.map_err(|_| format!(
        "{} {} timed out after {} seconds. Check the internet connection and retry. Details: {}",
        program, args.join(" "), limit.as_secs(), String::from_utf8_lossy(&err).trim()
    ))?.map_err(|e| format!("{program} could not finish: {e}"))?;
    if status.success() {
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    } else {
        // Tools like the app's login gate explain themselves on stdout.
        let out = String::from_utf8_lossy(&out);
        let lines: Vec<&str> = out.trim().lines().collect();
        let tail = lines[lines.len().saturating_sub(15)..].join("\n");
        let details = [tail.trim(), String::from_utf8_lossy(&err).trim()].iter()
            .filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("\n");
        Err(format!("{} {} failed: {}", program, args.join(" "), details))
    }
}

async fn report_progress<T>(
    label: &str,
    progress: &impl Fn(&str),
    operation: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    progress(label);
    let started = tokio::time::Instant::now();
    tokio::pin!(operation);
    loop {
        tokio::select! {
            result = &mut operation => {
                match &result {
                    Ok(_) => progress(&format!("Done: {label} ({}s)", started.elapsed().as_secs())),
                    Err(_) => progress(&format!("Failed: {label} ({}s)", started.elapsed().as_secs())),
                }
                return result;
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                progress(&format!("Still working: {label} — {}s elapsed", started.elapsed().as_secs()));
            }
        }
    }
}

/// The checkout to work in: the configured workspace, or Builder's own
/// worktree of the app repository, created from origin/main when missing.
async fn ensure_workspace(
    settings: &Settings,
    env: &HashMap<String, String>,
    progress: &impl Fn(&str),
) -> Result<PathBuf, String> {
    let app_repo = settings.app_repo.as_deref();
    let workspace = planned_workspace(settings).ok_or("Choose the app repository (../app) Builder works on.")?;
    let distro = wsl_distro(settings);
    let distro = distro.as_deref();
    let git = |args: Vec<String>, cwd: PathBuf| async move {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_on(distro, "git", &args, &cwd, env, Duration::from_secs(120)).await
    };
    progress("@step workspace");
    if !workspace.exists() {
        let repo = app_repo.ok_or_else(|| format!("Workspace {} does not exist", workspace.display()))?;
        progress("Creating Builder's worktree of the app…");
        if let Err(error) = report_progress("Fetching the latest app code", progress,
            git(vec!["fetch".into(), "origin".into(), "main".into()], repo.to_path_buf())).await {
            progress(&format!("Could not refresh the app; trying the existing checkout. {error}"));
        }
        let path = command_path(&workspace, distro);
        let has_branch = git(["rev-parse", "--verify", "--quiet", "refs/heads/builder/work"].map(String::from).to_vec(), repo.to_path_buf())
            .await
            .is_ok();
        if has_branch {
            git(vec!["worktree".into(), "add".into(), path, "builder/work".into()], repo.to_path_buf()).await?;
        } else {
            git(vec!["worktree".into(), "add".into(), "-b".into(), "builder/work".into(), path, "origin/main".into()], repo.to_path_buf()).await?;
        }
    }
    // A WSL path stays as chosen: canonicalising it on Windows gives a \\?\UNC form.
    let workspace = if distro.is_some() { workspace } else {
        dunce::canonicalize(&workspace).map_err(|e| format!("Workspace {}: {}", workspace.display(), e))?
    };
    progress(&format!("Workspace: {}", workspace.display()));

    progress("@step setup");
    // npm writes node_modules/.package-lock.json last; without it the app's
    // setup has not finished. `make setup` installs Node (mise), runs npm ci
    // and checks the login; it is safe to run again.
    if !workspace.join(SETUP_DONE).is_file() {
        let setup = || run_on(distro, "make", &["setup"], &workspace, env, Duration::from_secs(20 * 60));
        report_progress("Running the app's setup (make setup): Node, packages and login; the first time takes a few minutes", progress, setup()).await
            .map_err(|e| format!("{e}\n\nThe app's setup failed. If it asks for a login, enter your API email and password in Builder's setup, \
                or run `make setup` in {} in a terminal.", workspace.display()))?;
    }
    progress("The app is set up.");
    Ok(workspace)
}

/// The /api/mcp login: VA_MCP_BASIC from Builder's or the shell's
/// environment, else what `MCP_AUTH=1 bin/dev-status.sh` prints in api_dir.
async fn api_login(settings: &Settings, env: &HashMap<String, String>) -> Option<String> {
    if let Some(b) = std::env::var("VA_MCP_BASIC").ok().or_else(|| env.get("VA_MCP_BASIC").cloned()) {
        if !b.is_empty() {
            return Some(b);
        }
    }
    let api_dir = settings.api_dir.as_deref()?;
    let mut env = env.clone();
    env.insert("MCP_AUTH".into(), "1".into());
    run("bin/dev-status.sh", &[], api_dir, &env).await.ok().filter(|b| !b.is_empty())
}

/// Start Builder from settings. `progress` receives short status lines.
pub async fn start(settings: &Settings, progress: impl Fn(&str)) -> Result<Running, String> {
    start_with_api_credential(settings, None, progress).await
}

/// The shell environment a menu-launched app lacks (nvm, mise, Homebrew).
pub async fn shell_env(settings: &Settings) -> HashMap<String, String> {
    let env_dir = settings.app_repo.clone().or_else(|| settings.workspace.clone())
        .filter(|p| p.is_dir()).or_else(dirs::home_dir).unwrap_or_else(std::env::temp_dir);
    let distro = wsl_distro(settings);
    let dir = command_path(&env_dir, distro.as_deref());
    crate::shell_env::capture_shell_env(&dir, distro.as_deref()).await.unwrap_or_else(|e| {
        warn!("Shell environment capture failed, using the process environment: {}", e);
        std::env::vars().collect()
    })
}

/// Credentials supplied by onboarding are kept in memory, outside settings and agent environment.
pub async fn start_with_api_credential(settings: &Settings, login: Option<ApiLogin>, progress: impl Fn(&str)) -> Result<Running, String> {
    progress("@step check");
    progress("Reading your shell environment…");
    let mut env = shell_env(settings).await;
    if let Some(l) = &login {
        env.insert("TEST_EMAIL".into(), l.email.clone());
        env.insert("TEST_PASSWORD".into(), l.password.clone());
    }
    add_saved_login(settings, &mut env);
    let api_url = settings
        .api_url
        .clone()
        .or_else(|| settings.api_dir.as_deref().and_then(api_url_from_dir));
    // Vite gives process environment priority over existing .env files, and
    // the login gate in `make setup` signs in to it: set before setup.
    if let Some(api) = &api_url {
        env.insert("VITE_API_BASE_URL".into(), api.clone());
    }
    let checks = preflight(settings, &env).await;
    for check in &checks {
        progress(&check.line());
    }
    let failed: Vec<String> = checks.iter().filter(|c| c.level == Level::Fail).map(Check::line).collect();
    if !failed.is_empty() {
        return Err(format!("Pre-flight checks failed:\n{}", failed.join("\n")));
    }
    // For the app's login gate only; removed from the agent's environment below.
    let basic = match &login {
        Some(l) => Some(api_credential(&l.email, &l.password)?),
        None => None,
    };

    let workspace = ensure_workspace(settings, &env, &progress).await?;

    // The app's API proxy uses the same API as the agent. Written once; an
    // existing .env is never changed.
    if let Some(api) = &api_url {
        let dotenv = workspace.join(".env");
        if !dotenv.exists() {
            let _ = std::fs::write(&dotenv, format!("VITE_API_BASE_URL={}\n", api));
        }
    }
    let mut client_url = settings.client_url.trim_end_matches('/').to_string();
    let mut preview_url = settings.preview_url.clone().unwrap_or_else(|| client_url.clone());
    for (name, url) in [("client_url", &client_url), ("preview_url", &preview_url)] {
        if !valid_origin(url) {
            return Err(format!("{} must be an https origin or http://localhost[:port], got {}", name, url));
        }
    }

    let mut dev_server = None;
    progress("@step app");
    let app_answers = settings.start_app && answers(&client_url).await;
    if app_answers {
        progress(&format!("An app already answers at {client_url}; using it. If that is not this workspace's dev server (for example one running in WSL), stop it and restart Builder."));
    }
    if settings.start_app && !app_answers {
        let wanted = url_port(&client_url).unwrap_or(3000);
        let port = free_port(wanted).await.ok_or_else(|| format!("No free port between {wanted} and {} for the app dev server.", wanted + 19))?;
        if port != wanted {
            progress(&format!("Port {wanted} cannot be used on this computer (in use, or held by the system after WSL used it); the app will run on port {port}."));
            if preview_url == client_url { preview_url = with_port(&preview_url, port); }
            client_url = with_port(&client_url, port);
        }
        progress(&format!("Starting the app (make dev PORT={port})…"));
        dev_server = Some(start_dev_server(&workspace, env.clone(), port, wsl_distro(settings).as_deref())?);
        let deadline = tokio::time::Instant::now() + DEV_SERVER_WAIT;
        let mut next_update = tokio::time::Instant::now();
        while !answers(&client_url).await {
            if let Some(status) = dev_server.as_mut().unwrap().child.try_wait()
                .map_err(|e| format!("Could not check the app process: {e}"))? {
                let log = std::fs::read_to_string(state_dir().join("app.log")).unwrap_or_default();
                let tail: Vec<&str> = log.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
                return Err(format!("make dev stopped before the app was ready ({status}):\n{}\n\nSee {}",
                    tail.join("\n"), state_dir().join("app.log").display()));
            }
            if tokio::time::Instant::now() >= next_update {
                progress(&format!("Waiting for the app at {client_url}; startup limit is {} seconds. Details: {}",
                    DEV_SERVER_WAIT.as_secs(), state_dir().join("app.log").display()));
                next_update = tokio::time::Instant::now() + Duration::from_secs(5);
            }
            if tokio::time::Instant::now() > deadline {
                return Err(format!(
                    "The app dev server did not answer at {} within {}s; see {}",
                    client_url,
                    DEV_SERVER_WAIT.as_secs(),
                    state_dir().join("app.log").display()
                ));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    progress("@step bot");
    let bot = match &settings.bot_dir {
        None => {
            progress("No voice bot folder chosen.");
            None
        }
        Some(dir) => Some(start_bot(dir, &env, wsl_distro(settings), &progress).await?),
    };

    let mut api_mcp = None;
    progress("@step api");
    progress("Checking the API connection…");
    let api_note = match &api_url {
        None => "API tools off (no api_url or api_dir in settings)".to_string(),
        Some(api_url) if !valid_origin(api_url) => format!("API tools off: invalid api_url {}", api_url),
        Some(api_url) => match api_mcp_server(api_url, match basic {
            Some(ref value) => Some(value.clone()),
            None => api_login(settings, &env).await,
        }.as_deref()).await {
            Ok(api) => {
                let note = format!("API tools (read-only): {}", api.url);
                api_mcp = Some(api);
                note
            }
            Err(why) => format!("API tools off: {}", why),
        },
    };

    progress(&api_note);
    progress("@step server");
    progress("Starting Builder's local connection…");

    let port = free_port(settings.port).await
        .ok_or_else(|| format!("No free port between {} and {} for Builder.", settings.port, settings.port + 19))?;
    if port != settings.port {
        progress(&format!("Port {} is busy (another Builder, possibly running in WSL); using port {port}.", settings.port));
    }
    let distro = wsl_distro(settings);
    let mut agent_env: HashMap<String, String> = env.into_iter()
        .filter(|(k, _)| !matches!(k.as_str(), "VA_MCP_BASIC" | "TEST_EMAIL" | "TEST_PASSWORD")).collect();
    if distro.is_some() {
        agent_env = wsl_env(&agent_env);
    }
    // With a bot, the agent changes the bot; it still runs from the app's
    // packages, where Node and the agent are installed.
    let agent_dir = settings.bot_dir.clone().unwrap_or_else(|| workspace.clone());
    let mut agent = agent_command(&settings.agent_command, &workspace);
    if settings.bot_dir.is_some() {
        agent = format!("cd {} && exec {}", sh_quote(&command_path(&workspace, distro.as_deref())), agent);
    }
    let config = BuilderConfig {
        port,
        secret: BuilderConfig::new_secret(),
        // The agent runs in the distro, so it works in the Linux path.
        workspace: PathBuf::from(command_path(&agent_dir, distro.as_deref())),
        client_url,
        agent_command: agent,
        agent_env,
        wsl_distro: distro,
        mcp_servers: Vec::new(),
        api_mcp,
        api_url,
        preview_url,
        bot: bot.clone(),
    };
    let url = format!("http://127.0.0.1:{}/?secret={}", config.port, config.secret);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", config.port))
        .await
        .map_err(|e| format!("Port {} is busy ({}). Is Builder already running?", config.port, e))?;
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        if let Err(e) = builder::serve_on(listener, config, async {
            let _ = rx.await;
        })
        .await
        {
            warn!("Builder server stopped: {}", e);
        }
    });
    info!("Builder started; {}", api_note);

    Ok(Running {
        url,
        workspace,
        api_note,
        shutdown: Some(tx),
        server: Some(server),
        dev_server,
        bot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn hung_command_is_stopped_at_the_deadline() {
        let env = HashMap::from([("PATH".into(), "/usr/bin:/bin".into())]);
        let started = std::time::Instant::now();
        let error = run_with_timeout("sh", &["-c", "sleep 30"], Path::new("/tmp"), &env,
            Duration::from_millis(100)).await.unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(3));
    }


    #[tokio::test]
    async fn progress_reports_waiting_and_failure() {
        let lines = std::sync::Mutex::new(Vec::new());
        let result: Result<(), String> = report_progress("Test download", &|line| {
            lines.lock().unwrap().push(line.to_owned());
        }, async {
            tokio::time::sleep(Duration::from_millis(5100)).await;
            Err("network unavailable".into())
        }).await;
        assert_eq!(result.unwrap_err(), "network unavailable");
        let lines = lines.into_inner().unwrap();
        assert_eq!(lines[0], "Test download");
        assert!(lines.iter().any(|line| line.starts_with("Still working: Test download")));
        assert!(lines.last().unwrap().starts_with("Failed: Test download"));
    }



    #[test]
    fn settings_round_trip_and_defaults() {
        let parsed: Settings = toml::from_str("workspace = \"/w\"\napi_url = \"http://127.0.0.1:5055\"\n").unwrap();
        assert_eq!(parsed.workspace, Some(PathBuf::from("/w")));
        assert_eq!(parsed.client_url, "http://localhost:3000");
        assert!(parsed.start_app);
        let text = toml::to_string_pretty(&parsed).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), parsed);
        assert!(toml::from_str::<Settings>("tokn = \"x\"").is_err());
    }

    #[test]
    fn worktree_and_api_defaults() {
        assert_eq!(default_worktree(Path::new("/w/app")), PathBuf::from("/w/app-builder"));
        let dir = std::env::temp_dir().join(format!("builder-api-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".env"), "A=1\nVA_WEB_PORT=5000\nVA_WEB_PORT=5055\n").unwrap();
        assert_eq!(api_url_from_dir(&dir).as_deref(), Some("http://127.0.0.1:5055"));
        std::fs::write(dir.join(".env"), "VA_WEB_PORT=evil.example\n").unwrap();
        assert_eq!(api_url_from_dir(&dir), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hosted_onboarding_clears_previous_local_api() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/builder-client")).unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        std::fs::write(dir.path().join("src/builder-client/main.jsx"), "").unwrap();
        let mut previous = Settings::default();
        previous.api_dir = Some(PathBuf::from("old-local-api"));
        let settings = onboarding_settings(previous, dir.path().to_str().unwrap(), "https://switch.voipappz.io/", "").unwrap();
        assert_eq!(settings.api_url.as_deref(), Some("https://switch.voipappz.io"));
        assert!(settings.api_dir.is_none());
        assert!(settings.workspace.is_none());
    }


    #[test]
    fn wsl_settings_give_the_distro_and_linux_paths() {
        let settings = Settings { app_repo: Some(PathBuf::from(r"\\wsl.localhost\Debian\home\nir\voipappz\app")), ..Settings::default() };
        assert_eq!(wsl_distro(&settings).as_deref(), Some("Debian"));
        assert_eq!(command_path(Path::new(r"\\wsl.localhost\Debian\home\nir\voipappz\app-builder"), Some("Debian")), "/home/nir/voipappz/app-builder");
        assert_eq!(command_path(Path::new("/home/nir/app"), None), "/home/nir/app");
        assert_eq!(wsl_distro(&Settings { app_repo: Some(PathBuf::from("/home/nir/app")), ..Settings::default() }), None);
    }

    #[test]
    fn wsl_version_from_the_kernel() {
        assert!(wsl2_kernel("6.6.87.2-microsoft-standard-WSL2"));
        assert!(wsl2_kernel("5.15.167.4-microsoft-standard-WSL2\n"));
        assert!(!wsl2_kernel("4.4.0-19041-Microsoft"));
        assert!(!wsl2_kernel(""));
    }

    #[test]
    fn shell_quoting_and_wsl_env() {
        assert_eq!(sh_quote("make"), "make");
        assert_eq!(sh_quote("PORT=3001"), "PORT=3001");
        assert_eq!(sh_quote("it's here"), "'it'\\''s here'");
        assert_eq!(sh_quote(""), "''");
        let env = HashMap::from([("PATH".to_string(), "/usr/bin".to_string()), ("TEST_EMAIL".into(), "a@b".into()),
            ("WINDIR".into(), r"C:\Windows".into()), ("ANTHROPIC_MODEL".into(), "x".into())]);
        let mut keys: Vec<String> = wsl_env(&env).into_keys().collect();
        keys.sort();
        assert_eq!(keys, ["ANTHROPIC_MODEL", "PATH", "TEST_EMAIL"]);
    }

    #[test]
    fn checks_render_with_their_level() {
        assert_eq!(Check::new("git", Level::Ok, "found").line(), "✓ git: found");
        assert_eq!(Check::new("Disk space", Level::Fail, "1 MB free").line(), "✕ Disk space: 1 MB free");
    }

    #[test]
    fn free_space_is_measured() {
        assert!(free_bytes(&std::env::temp_dir()).is_some_and(|b| b > 0));
        assert!(free_bytes(&std::env::temp_dir().join("not/there/yet")).is_some());
    }

    #[tokio::test]
    async fn preflight_flags_a_folder_that_is_not_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings { app_repo: Some(dir.path().to_path_buf()), ..Settings::default() };
        let env: HashMap<String, String> = std::env::vars().collect();
        let checks = preflight(&settings, &env).await;
        let folder = checks.iter().find(|c| c.label == "App folder").unwrap();
        assert_eq!(folder.level, Level::Fail, "{folder:?}");
        std::fs::write(dir.path().join("Makefile"), "dev:\n").unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        let checks = preflight(&settings, &env).await;
        assert_eq!(checks.iter().find(|c| c.label == "App folder").unwrap().level, Level::Ok);
        assert!(checks.iter().any(|c| c.label == "make"));
    }

    async fn login_check(settings: &Settings, env: &HashMap<String, String>) -> Level {
        preflight(settings, env).await.into_iter().find(|c| c.label == "API login").unwrap().level
    }

    #[tokio::test]
    async fn preflight_needs_an_api_login() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("app");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("Makefile"), "dev:\n").unwrap();
        std::fs::write(repo.join("package.json"), "{}").unwrap();
        let mut settings = Settings { app_repo: Some(repo.clone()), start_app: false, ..Settings::default() };
        let env: HashMap<String, String> = std::env::vars().filter(|(k, _)| !k.starts_with("TEST_")).collect();
        let saved = |settings: &Settings| { let mut env = env.clone(); add_saved_login(settings, &mut env); env };
        assert_eq!(login_check(&settings, &saved(&settings)).await, Level::Fail);
        // A cached token alone does not get past the app's login gate.
        std::fs::write(repo.join(".env.token"), "t").unwrap();
        std::fs::write(repo.join(".env"), "TEST_EMAIL=\nTEST_PASSWORD=x\n").unwrap();
        assert_eq!(login_check(&settings, &saved(&settings)).await, Level::Fail);
        // The app repository's login also serves Builder's new worktree.
        std::fs::write(repo.join(".env"), "TEST_EMAIL=a@b.c\nTEST_PASSWORD=\"x\"\n").unwrap();
        let env_with_login = saved(&settings);
        assert_eq!(env_with_login["TEST_PASSWORD"], "x");
        assert_eq!(login_check(&settings, &env_with_login).await, Level::Ok);
        // Set up, and no make dev: no login gate runs, so none is needed.
        std::fs::remove_file(repo.join(".env")).unwrap();
        let worktree = default_worktree(&repo);
        std::fs::create_dir_all(worktree.join("node_modules")).unwrap();
        std::fs::write(worktree.join(SETUP_DONE), "{}").unwrap();
        assert!(preflight(&settings, &env).await.iter().all(|c| c.label != "API login"));
        settings.start_app = true;
        assert_eq!(login_check(&settings, &env).await, Level::Fail);
    }

    #[test]
    fn the_apps_own_agent_replaces_npx() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(agent_command(DEFAULT_AGENT_COMMAND, dir.path()), DEFAULT_AGENT_COMMAND);
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/.bin")).unwrap();
        std::fs::write(dir.path().join("bin/npm.sh"), "").unwrap();
        std::fs::write(dir.path().join("node_modules/.bin/claude-agent-acp"), "").unwrap();
        assert_eq!(agent_command(DEFAULT_AGENT_COMMAND, dir.path()), APP_AGENT_COMMAND);
        assert_eq!(agent_command("my-agent --acp", dir.path()), "my-agent --acp");
    }

    #[test]
    fn urls_change_port() {
        assert_eq!(url_port("http://localhost:3000"), Some(3000));
        assert_eq!(url_port("https://switch.voipappz.io"), None);
        assert_eq!(with_port("http://localhost:3000", 3001), "http://localhost:3001");
        assert_eq!(with_port("http://localhost", 3001), "http://localhost:3001");
        assert_eq!(with_port("http://admin.localhost:3000/", 3005), "http://admin.localhost:3005");
    }

    #[tokio::test]
    async fn free_port_skips_a_bound_port() {
        let held = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let taken = held.local_addr().unwrap().port();
        let port = free_port(taken).await.unwrap();
        assert_ne!(port, taken);
        assert!(port > taken && port < taken + 20);
    }


    #[test]
    fn onboarding_refuses_wsl_and_network_paths() {
        assert!(is_network_path(r"\\wsl.localhost\Debian\home\nir\voipappz\app"));
        assert!(is_network_path(r"\\wsl$\Ubuntu\home\nir\app"));
        assert!(is_network_path(r"\\server\share\app"));
        assert!(!is_network_path(r"C:\Users\nir\voipappz-app"));
        assert!(!is_network_path("/home/nir/voipappz/app"));
        let share = onboarding_settings(Settings::default(), r"\\server\share\app", "https://switch.voipappz.io", "").unwrap_err();
        assert!(share.contains("network share"), "{share}");
        // A WSL checkout gets past the share check (here it then lacks the app's files).
        let wsl = onboarding_settings(Settings::default(), r"\\wsl.localhost\Debian\home\nir\voipappz\app", "https://switch.voipappz.io", "").unwrap_err();
        assert!(!wsl.contains("network share"), "{wsl}");
    }

    #[test]
    fn onboarding_credentials_require_both_fields() {
        assert!(api_credential("", "password").is_err());
        assert!(api_credential("person@example.com", "").is_err());
        assert!(api_credential("person:other", "password").is_err());
    }

    #[test]
    fn origins() {
        assert!(valid_origin("https://switch.voipappz.io"));
        assert!(valid_origin("http://localhost:3000/"));
        assert!(valid_origin("http://127.0.0.1:5055"));
        assert!(!valid_origin("http://switch.voipappz.io"));
        assert!(!valid_origin("https://switch.voipappz.io/builder/client.js"));
        assert!(!valid_origin("http://localhost.evil.example"));
        assert!(!valid_origin("https://user:password@switch.voipappz.io"));
        assert!(!valid_origin("https://switch.voipappz.io?token=secret"));
        assert!(!valid_origin("https://switch.voipappz.io#fragment"));
    }
}
