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
    let url = url.trim_end_matches('/');
    let local = ["http://localhost", "http://127.0.0.1"]
        .iter()
        .any(|p| url == *p || url.starts_with(&format!("{}:", p)));
    (url.starts_with("https://") || local) && url.matches('/').count() == 2
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
    let basic = match basic {
        Some(b) if !b.is_empty() => b,
        _ => return Err("VA_MCP_BASIC is not set (in voipappz-api: make tenant, then MCP_AUTH=1 bin/dev-status.sh)".into()),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;
    match client.get(format!("{}/health", api_url)).send().await {
        Ok(r) if r.status().is_success() => {}
        _ => return Err(format!("{}/health does not answer (in voipappz-api: make dev)", api_url)),
    }
    let listed = client
        .post(format!("{}/api/mcp", api_url))
        .header("Authorization", format!("Basic {}", basic))
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        .send()
        .await;
    match listed {
        Ok(r) if r.status().is_success() => {}
        Ok(r) => return Err(format!("{}/api/mcp answered {} for the VA_MCP_BASIC login", api_url, r.status())),
        Err(_) => return Err(format!("{}/api/mcp does not answer", api_url)),
    }
    Ok(ApiMcp {
        url: format!("{}/api/mcp", api_url),
        basic: basic.to_string(),
    })
}

/// A running Builder. Dropping it stops the dev server it started.
pub struct Running {
    pub url: String,
    pub workspace: PathBuf,
    pub api_note: String,
    shutdown: Option<oneshot::Sender<()>>,
    server: Option<tokio::task::JoinHandle<()>>,
    dev_server: Option<ChildProcess>,
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
    }
}

fn start_dev_server(workspace: &Path, env: HashMap<String, String>) -> Result<ChildProcess, String> {
    if !workspace.join("node_modules").exists() {
        return Err(format!(
            "{} has no node_modules; run npm install there first",
            workspace.display()
        ));
    }
    let log_dir = state_dir();
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let log = std::fs::File::create(log_dir.join("app.log")).map_err(|e| e.to_string())?;
    let log_err = log.try_clone().map_err(|e| e.to_string())?;
    let mut command = create_shell_command("npm run dev", env, &workspace.to_string_lossy(), None);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    spawn_command(command).map_err(|e| format!("could not start npm run dev: {}", e))
}

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
    let output = tokio::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| format!("{} could not run: {}", program, e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "{} {} failed: {}",
            program,
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
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
    let workspace = match (&settings.workspace, app_repo) {
        (Some(w), _) => w.clone(),
        (None, Some(repo)) => default_worktree(repo),
        (None, None) => return Err("Choose the app repository (../app) Builder works on.".into()),
    };
    if !workspace.exists() {
        let repo = app_repo.ok_or_else(|| format!("Workspace {} does not exist", workspace.display()))?;
        progress("Creating Builder's worktree of the app…");
        let _ = run("git", &["fetch", "origin", "main"], repo, env).await;
        let path = workspace.to_string_lossy().into_owned();
        let has_branch = run("git", &["rev-parse", "--verify", "--quiet", "refs/heads/builder/work"], repo, env)
            .await
            .is_ok();
        if has_branch {
            run("git", &["worktree", "add", &path, "builder/work"], repo, env).await?;
        } else {
            run("git", &["worktree", "add", "-b", "builder/work", &path, "origin/main"], repo, env).await?;
        }
    }
    let workspace = std::fs::canonicalize(&workspace)
        .map_err(|e| format!("Workspace {}: {}", workspace.display(), e))?;

    if !workspace.join("node_modules").exists() {
        let shared = app_repo.map(|r| r.join("node_modules")).filter(|p| p.is_dir());
        let same_lock = app_repo.is_some_and(|r| {
            std::fs::read(r.join("package-lock.json")).ok() == std::fs::read(workspace.join("package-lock.json")).ok()
        });
        match shared {
            #[cfg(unix)]
            Some(modules) if same_lock => {
                std::os::unix::fs::symlink(&modules, workspace.join("node_modules"))
                    .map_err(|e| format!("Could not link node_modules: {}", e))?;
            }
            _ => {
                progress("Installing the app's packages (npm ci)…");
                run("npm", &["ci"], &workspace, env).await?;
            }
        }
    }
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
    // A menu-launched app does not get the terminal's PATH (nvm, mise,
    // Homebrew); use what a login shell would have.
    progress("Reading your shell environment…");
    let env_dir = settings
        .app_repo
        .clone()
        .or_else(|| settings.workspace.clone())
        .filter(|p| p.is_dir())
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir);
    let env = crate::shell_env::capture_shell_env(&env_dir.to_string_lossy(), None)
        .await
        .unwrap_or_else(|e| {
            warn!("Shell environment capture failed, using the process environment: {}", e);
            std::env::vars().collect()
        });

    let workspace = ensure_workspace(settings, &env, &progress).await?;
    let api_url = settings
        .api_url
        .clone()
        .or_else(|| settings.api_dir.as_deref().and_then(api_url_from_dir));

    // The app's API proxy uses the same API as the agent. Written once; an
    // existing .env is never changed.
    if let Some(api) = &api_url {
        let dotenv = workspace.join(".env");
        if !dotenv.exists() {
            let _ = std::fs::write(&dotenv, format!("VITE_API_BASE_URL={}\n", api));
        }
    }
    let client_url = settings.client_url.trim_end_matches('/').to_string();
    let preview_url = settings.preview_url.clone().unwrap_or_else(|| client_url.clone());
    for (name, url) in [("client_url", &client_url), ("preview_url", &preview_url)] {
        if !valid_origin(url) {
            return Err(format!("{} must be an https origin or http://localhost[:port], got {}", name, url));
        }
    }

    let mut dev_server = None;
    if settings.start_app && !answers(&client_url).await {
        progress("Starting the app dev server (npm run dev)…");
        dev_server = Some(start_dev_server(&workspace, env.clone())?);
        let deadline = tokio::time::Instant::now() + DEV_SERVER_WAIT;
        while !answers(&client_url).await {
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

    let mut api_mcp = None;
    let api_note = match &api_url {
        None => "API tools off (no api_url or api_dir in settings)".to_string(),
        Some(api_url) if !valid_origin(api_url) => format!("API tools off: invalid api_url {}", api_url),
        Some(api_url) => match api_mcp_server(api_url, api_login(settings, &env).await.as_deref()).await {
            Ok(api) => {
                let note = format!("API tools (read-only): {}", api.url);
                api_mcp = Some(api);
                note
            }
            Err(why) => format!("API tools off: {}", why),
        },
    };

    let config = BuilderConfig {
        port: settings.port,
        secret: BuilderConfig::new_secret(),
        workspace: workspace.clone(),
        client_url,
        agent_command: settings.agent_command.clone(),
        agent_env: env.into_iter().filter(|(k, _)| k != "VA_MCP_BASIC").collect(),
        mcp_servers: Vec::new(),
        api_mcp,
        api_url,
        preview_url,
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn origins() {
        assert!(valid_origin("https://switch.voipappz.io"));
        assert!(valid_origin("http://localhost:3000/"));
        assert!(valid_origin("http://127.0.0.1:5055"));
        assert!(!valid_origin("http://switch.voipappz.io"));
        assert!(!valid_origin("https://switch.voipappz.io/builder/client.js"));
        assert!(!valid_origin("http://localhost.evil.example"));
    }
}
