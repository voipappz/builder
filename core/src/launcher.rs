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
    let limit = if args == ["--version"] { Duration::from_secs(20) }
        else if program == "git" { Duration::from_secs(120) }
        else { Duration::from_secs(15 * 60) };
    run_with_timeout(program, args, cwd, env, limit).await
}

async fn run_with_timeout(program: &str, args: &[&str], cwd: &Path, env: &HashMap<String, String>, limit: Duration) -> Result<String, String> {
    use tokio::io::AsyncReadExt;
    // npm ships as npm.cmd on Windows and must run through cmd.exe.
    #[cfg(windows)]
    let mut command = if program == "npm" {
        let mut command = tokio::process::Command::new("cmd.exe");
        command.args(["/d", "/c", "npm"]);
        command
    } else {
        tokio::process::Command::new(program)
    };
    #[cfg(not(windows))]
    let mut command = tokio::process::Command::new(program);
    command.args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
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
        Err(format!(
            "{} {} failed: {}",
            program,
            args.join(" "),
            String::from_utf8_lossy(&err).trim()
        ))
    }
}

fn node_pin(repo: &Path) -> String {
    std::fs::read_to_string(repo.join("mise.toml"))
        .ok()
        .and_then(|text| toml::from_str::<toml::Value>(&text).ok())
        .and_then(|value| value.get("tools")?.get("node")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| "22".into())
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

async fn install_mise(
    destination: &Path,
    script: &str,
    env: &HashMap<String, String>,
) -> Result<(), String> {
    let parent = destination.parent().ok_or("Invalid mise installation path")?;
    tokio::fs::create_dir_all(parent).await.map_err(|e| format!("Create mise directory: {e}"))?;
    let temporary = parent.join(format!("install-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir(&temporary).await.map_err(|e| format!("Create mise installer directory: {e}"))?;
    let installer = temporary.join(if cfg!(windows) { "install.ps1" } else { "install.sh" });
    let result = async {
        tokio::fs::write(&installer, script).await.map_err(|e| format!("Save mise installer: {e}"))?;
        let mut install_env = env.clone();
        install_env.insert("MISE_INSTALL_PATH".into(), destination.to_string_lossy().into_owned());
        install_env.insert("MISE_INSTALL_MODIFY_PATH".into(), "0".into());
        install_env.insert("MISE_INSTALL_HELP".into(), "0".into());
        let path = installer.to_string_lossy();
        #[cfg(windows)]
        run("powershell.exe", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", &path], parent, &install_env).await?;
        #[cfg(not(windows))]
        run("sh", &[&path], parent, &install_env).await?;
        run(&destination.to_string_lossy(), &["--version"], parent, env).await
            .map_err(|e| format!("mise installation did not produce a working executable: {e}"))?;
        Ok(())
    }.await;
    let _ = tokio::fs::remove_file(&installer).await;
    let _ = tokio::fs::remove_dir(&temporary).await;
    result
}

async fn ensure_mise(
    repo: &Path,
    env: &HashMap<String, String>,
    progress: &impl Fn(&str),
) -> Result<PathBuf, String> {
    let executable = if cfg!(windows) { "mise.exe" } else { "mise" };
    let managed = state_dir().join("tools").join(executable);
    let existing = which::which_in(executable, env.get("PATH"), repo).ok()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/bin").join(executable)).filter(|p| p.is_file()))
        .or_else(|| managed.is_file().then(|| managed.clone()));
    if let Some(path) = existing {
        progress(&format!("Using mise: {}", path.display()));
        return Ok(path);
    }

    progress("Installing mise for Builder (first run)…");
    let url = if cfg!(windows) { "https://mise.run/powershell" } else { "https://mise.run" };
    let script = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build().map_err(|e| format!("Create mise download client: {e}"))?
        .get(url).send().await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Could not download the mise installer. Check your internet connection and retry: {e}"))?
        .text().await.map_err(|e| format!("Read mise installer: {e}"))?;
    progress("mise installer downloaded successfully.");
    report_progress("Installing mise; downloading and verifying its release", progress, install_mise(&managed, &script, env)).await
        .map_err(|e| format!("Could not install mise for Builder: {e}"))?;
    Ok(managed)
}

async fn ensure_node(
    repo: &Path,
    env: &mut HashMap<String, String>,
    progress: &impl Fn(&str),
) -> Result<(), String> {
    let pin = node_pin(repo);
    progress(&format!("Checking Node.js and npm (required Node version: {pin})…"));
    let version = run("node", &["--version"], repo, env).await.unwrap_or_default();
    let major = version.trim_start_matches('v').split('.').next().and_then(|v| v.parse::<u32>().ok());
    let required = pin.split('.').next().and_then(|v| v.parse::<u32>().ok()).unwrap_or(22);
    if major.is_some_and(|v| v >= required) && run("npm", &["--version"], repo, env).await.is_ok() {
        progress(&format!("Node {version} and npm are already available."));
        return Ok(());
    }
    let mise = ensure_mise(repo, env, progress).await?;
    let mise = mise.to_string_lossy();
    let tool = format!("node@{pin}");
    // We already read the tool pin. Bootstrap must also work in a fresh,
    // untrusted checkout without loading its mise hooks or environment.
    let mut mise_env = env.clone();
    mise_env.insert("MISE_NO_CONFIG".into(), "1".into());
    progress(&format!("Installing Node {pin} and npm with mise…"));
    report_progress("Installing Node.js and npm with mise; first setup may take several minutes", progress,
        run(&mise, &["install", &tool], repo, &mise_env)).await?;
    let location = run(&mise, &["where", &tool], repo, &mise_env).await?;
    let bin = PathBuf::from(location);
    #[cfg(not(windows))]
    let bin = bin.join("bin");
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(env.get("PATH").map(String::as_str).unwrap_or_default()));
    let path = std::env::join_paths(paths).map_err(|e| format!("Node PATH: {e}"))?;
    env.insert("PATH".into(), path.to_string_lossy().into_owned());
    run("npm", &["--version"], repo, env).await
        .map_err(|e| format!("mise installed Node but npm is still unavailable: {e}"))?;
    progress("Node.js and npm are ready.");
    Ok(())
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
        if let Err(error) = report_progress("Fetching the latest app code", progress,
            run("git", &["fetch", "origin", "main"], repo, env)).await {
            progress(&format!("Could not refresh the app; trying the existing checkout. {error}"));
        }
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
    progress(&format!("Workspace: {}", workspace.display()));

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
                report_progress("Installing app packages with npm ci; this can take several minutes", progress,
                    run("npm", &["ci"], &workspace, env)).await?;
            }
        }
    }
    progress("App packages are ready.");
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
    let mut env = crate::shell_env::capture_shell_env(&env_dir.to_string_lossy(), None)
        .await
        .unwrap_or_else(|e| {
            warn!("Shell environment capture failed, using the process environment: {}", e);
            std::env::vars().collect()
        });

    ensure_node(&env_dir, &mut env, &progress).await?;
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
        let mut next_update = tokio::time::Instant::now();
        while !answers(&client_url).await {
            if let Some(status) = dev_server.as_mut().unwrap().child.try_wait()
                .map_err(|e| format!("Could not check the app process: {e}"))? {
                return Err(format!("The app stopped before it was ready ({status}). See {}",
                    state_dir().join("app.log").display()));
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

    let mut api_mcp = None;
    progress("Checking the API connection…");
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

    progress(&api_note);
    progress("Starting Builder's local connection…");

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

    #[cfg(windows)]
    #[tokio::test]
    async fn npm_cmd_works_with_a_custom_path() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("node tools");
        std::fs::create_dir(&bin).unwrap();
        std::fs::write(bin.join("npm.cmd"), "@echo off\r\necho 10.0.0\r\n").unwrap();
        let mut env: HashMap<String, String> = std::env::vars().filter(|(k, _)| !k.eq_ignore_ascii_case("PATH")).collect();
        env.insert("PATH".into(), bin.to_string_lossy().into_owned());
        assert_eq!(run("npm", &["--version"], dir.path(), &env).await.unwrap(), "10.0.0");
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

    #[cfg(unix)]
    #[tokio::test]
    async fn mise_bootstrap_installs_to_private_path_and_reports_failures() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("private tools/mise");
        let env = HashMap::from([("PATH".into(), "/usr/bin:/bin".into())]);
        let script = "#!/bin/sh\nset -eu\n[ \"$MISE_INSTALL_MODIFY_PATH\" = 0 ]\nprintf '#!/bin/sh\\necho mise-test\\n' > \"$MISE_INSTALL_PATH\"\nchmod 755 \"$MISE_INSTALL_PATH\"\n";
        install_mise(&destination, script, &env).await.unwrap();
        assert_eq!(run(destination.to_str().unwrap(), &["--version"], dir.path(), &env).await.unwrap(), "mise-test");
        assert_eq!(std::fs::read_dir(destination.parent().unwrap()).unwrap().count(), 1);

        let failed = dir.path().join("failed/mise");
        let error = install_mise(&failed, "echo download-failed >&2; exit 1", &env).await.unwrap_err();
        assert!(error.contains("download-failed"), "{error}");
        assert!(!failed.exists());
        assert_eq!(std::fs::read_dir(failed.parent().unwrap()).unwrap().count(), 0);
        let error = install_mise(&failed, "exit 0", &env).await.unwrap_err();
        assert!(error.contains("working executable"), "{error}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn missing_npm_uses_mise_and_exports_node_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        let node_bin = dir.path().join("runtime/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&node_bin).unwrap();
        std::fs::write(dir.path().join("mise.toml"), "[tools]\nnode = '22'\n").unwrap();
        let script = format!(
            "#!/bin/sh\n[ \"$MISE_NO_CONFIG\" = 1 ] || exit 7\n[ \"$2\" = node@22 ] || exit 9\ncase $1 in\ninstall) exit 0;;\nwhere) printf '%s\\n' '{}' ;;\n*) exit 8;;\nesac\n",
            dir.path().join("runtime").display()
        );
        for (path, contents) in [
            (bin.join("mise"), script.as_str()),
            (node_bin.join("npm"), "#!/bin/sh\necho 10.0.0\n"),
            (node_bin.join("node"), "#!/bin/sh\necho v22.0.0\n"),
        ] {
            std::fs::write(&path, contents).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut env = HashMap::from([("PATH".into(), bin.to_string_lossy().into_owned())]);
        ensure_node(dir.path(), &mut env, &|_| {}).await.unwrap();
        assert!(!env.contains_key("MISE_NO_CONFIG"));
        assert_eq!(run("node", &["--version"], dir.path(), &env).await.unwrap(), "v22.0.0");
        // A working runtime must not invoke mise again.
        std::fs::remove_file(bin.join("mise")).unwrap();
        ensure_node(dir.path(), &mut env, &|_| {}).await.unwrap();
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
    fn origins() {
        assert!(valid_origin("https://switch.voipappz.io"));
        assert!(valid_origin("http://localhost:3000/"));
        assert!(valid_origin("http://127.0.0.1:5055"));
        assert!(!valid_origin("http://switch.voipappz.io"));
        assert!(!valid_origin("https://switch.voipappz.io/builder/client.js"));
        assert!(!valid_origin("http://localhost.evil.example"));
    }
}
