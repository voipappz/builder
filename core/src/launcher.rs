//! Starting VoipAppz Builder from saved settings: the settings file, the
//! app's dev server, the API login check and the local server. The desktop
//! app uses this; the CLI shares the API check.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{info, warn};

use crate::builder::{self, BuilderConfig};
use crate::command::{create_shell_command, spawn_command, ChildProcess};

pub const DEFAULT_AGENT_COMMAND: &str = "npx -y @agentclientprotocol/claude-agent-acp@0.94.0";
const DEV_SERVER_WAIT: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// The app checkout the agent works in (a git worktree of the app).
    pub workspace: Option<PathBuf>,
    /// Where the chat client loads from: the app's dev server or a deployment.
    pub client_url: String,
    /// The app shown in the previews; defaults to `client_url`.
    pub preview_url: Option<String>,
    /// VoipAppz API whose /api/mcp tools the agent gets. The login comes
    /// from VA_MCP_BASIC in the user's shell environment.
    pub api_url: Option<String>,
    /// Start `npm run dev` in the workspace when the client does not answer.
    pub start_app: bool,
    pub port: u16,
    pub agent_command: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            workspace: None,
            client_url: "http://localhost:3000".to_string(),
            preview_url: None,
            api_url: None,
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

/// The `/api/mcp` server entry for ACP, after checking the API answers and
/// accepts the login. The error says why the tools are off.
pub async fn api_mcp_server(api_url: &str, basic: Option<&str>) -> Result<Value, String> {
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
    Ok(json!({
        "type": "http",
        "name": "voipappz",
        "url": format!("{}/api/mcp", api_url),
        "headers": [{"name": "Authorization", "value": format!("Basic {}", basic)}]
    }))
}

/// A running Builder. Dropping it stops the dev server it started.
pub struct Running {
    pub url: String,
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

/// Start Builder from settings. `progress` receives short status lines.
pub async fn start(settings: &Settings, progress: impl Fn(&str)) -> Result<Running, String> {
    let workspace = settings
        .workspace
        .clone()
        .ok_or("No workspace chosen. Choose the app checkout Builder should work in.")?;
    let workspace = std::fs::canonicalize(&workspace)
        .map_err(|e| format!("Workspace {}: {}", workspace.display(), e))?;
    let client_url = settings.client_url.trim_end_matches('/').to_string();
    let preview_url = settings.preview_url.clone().unwrap_or_else(|| client_url.clone());
    for (name, url) in [("client_url", &client_url), ("preview_url", &preview_url)] {
        if !valid_origin(url) {
            return Err(format!("{} must be an https origin or http://localhost[:port], got {}", name, url));
        }
    }

    // A menu-launched app does not get the terminal's PATH (nvm, mise,
    // Homebrew); use what a login shell in the workspace would have.
    progress("Reading your shell environment…");
    let env = crate::shell_env::capture_shell_env(&workspace.to_string_lossy(), None)
        .await
        .unwrap_or_else(|e| {
            warn!("Shell environment capture failed, using the process environment: {}", e);
            std::env::vars().collect()
        });

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

    let mut mcp_servers = Vec::new();
    let api_note = match &settings.api_url {
        None => "API tools off (no api_url in settings)".to_string(),
        Some(api_url) if !valid_origin(api_url) => format!("API tools off: invalid api_url {}", api_url),
        Some(api_url) => match api_mcp_server(api_url, env.get("VA_MCP_BASIC").map(String::as_str)).await {
            Ok(server) => {
                mcp_servers.push(server);
                format!("API tools: {}/api/mcp", api_url.trim_end_matches('/'))
            }
            Err(why) => format!("API tools off: {}", why),
        },
    };

    let config = BuilderConfig {
        port: settings.port,
        secret: BuilderConfig::new_secret(),
        workspace,
        client_url,
        agent_command: settings.agent_command.clone(),
        agent_env: env.into_iter().filter(|(k, _)| k != "VA_MCP_BASIC").collect(),
        mcp_servers,
        api_url: settings.api_url.clone(),
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
    fn origins() {
        assert!(valid_origin("https://switch.voipappz.io"));
        assert!(valid_origin("http://localhost:3000/"));
        assert!(valid_origin("http://127.0.0.1:5055"));
        assert!(!valid_origin("http://switch.voipappz.io"));
        assert!(!valid_origin("https://switch.voipappz.io/builder/client.js"));
        assert!(!valid_origin("http://localhost.evil.example"));
    }
}
