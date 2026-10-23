use builder_core::builder::{self, BuilderConfig};
use builder_core::launcher::{api_mcp_server, valid_origin, DEFAULT_AGENT_COMMAND};
use clap::Parser;
use std::path::PathBuf;


/// VoipAppz Builder: chat with a coding agent about an app checkout and see
/// it running.
#[derive(Parser, Debug)]
#[command(name = "voipappz-builder", version, about)]
struct Cli {
    /// The checkout the agent works in. Use a git worktree, not a checkout
    /// with other work in progress.
    #[arg(long)]
    workspace: PathBuf,

    #[arg(short, long, default_value_t = 9832)]
    port: u16,

    /// Where the chat client is loaded from: the deployed app, or a local
    /// app run such as http://localhost:3000.
    #[arg(long, env = "VOIPAPPZ_BUILDER_CLIENT_URL", default_value = "https://switch.voipappz.io")]
    client_url: String,

    /// The app's dev server, shown in the Admin and User previews.
    #[arg(long, env = "VOIPAPPZ_BUILDER_PREVIEW_URL", default_value = "http://localhost:3000")]
    preview_url: String,

    /// VoipAppz API whose /api/mcp tools the agent gets, e.g.
    /// http://127.0.0.1:5055. The login is read from VA_MCP_BASIC, as
    /// `make claude` in voipappz-api sets it.
    #[arg(long, env = "VOIPAPPZ_API_URL")]
    api_url: Option<String>,

    /// Command that starts the ACP agent.
    #[arg(long, env = "VOIPAPPZ_BUILDER_AGENT", default_value = DEFAULT_AGENT_COMMAND)]
    agent_command: String,

    /// Open the Builder page in the default browser once the server is up.
    #[arg(long)]
    open: bool,

    /// Write the page address (it carries the launch secret) to this file,
    /// readable only by the current user, so a launcher can reopen it.
    #[arg(long)]
    url_file: Option<PathBuf>,

    #[arg(short, long)]
    debug: bool,
}

fn write_private(path: &PathBuf, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    options.open(path)?.write_all(text.as_bytes())
}

fn open_in_browser(url: &str) {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/c", "start", "", url]).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(e) = result {
        eprintln!("  Could not open a browser: {}", e);
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(if cli.debug { "debug" } else { "info" })
        .init();

    let workspace = std::fs::canonicalize(&cli.workspace)
        .map_err(|e| format!("workspace {}: {}", cli.workspace.display(), e))?;
    if !workspace.is_dir() {
        return Err(format!("workspace {} is not a directory", workspace.display()).into());
    }
    for (name, url) in [("--client-url", &cli.client_url), ("--preview-url", &cli.preview_url)] {
        if !valid_origin(url) {
            return Err(format!("{} must be an https origin or http://localhost[:port], got {}", name, url).into());
        }
    }

    let mut mcp_servers = Vec::new();
    if let Some(api_url) = &cli.api_url {
        if !valid_origin(api_url) {
            return Err(format!("--api-url must be an https origin or http://localhost|127.0.0.1[:port], got {}", api_url).into());
        }
        match api_mcp_server(api_url, std::env::var("VA_MCP_BASIC").ok().as_deref()).await {
            Ok(server) => mcp_servers.push(server),
            Err(why) => eprintln!("  API tools off: {}", why),
        }
    }

    let config = BuilderConfig {
        port: cli.port,
        secret: BuilderConfig::new_secret(),
        workspace,
        client_url: cli.client_url.trim_end_matches('/').to_string(),
        agent_command: cli.agent_command,
        agent_env: std::env::vars().filter(|(k, _)| k != "VA_MCP_BASIC").collect(),
        mcp_servers,
        api_url: cli.api_url.clone(),
        preview_url: cli.preview_url,
    };

    let (admin, user) = config.preview_urls();
    eprintln!();
    eprintln!("VoipAppz Builder");
    eprintln!("  workspace  {}", config.workspace.display());
    eprintln!("  chat from  {}", config.client_url);
    eprintln!("  previews   {}  {}", admin, user);
    eprintln!(
        "  API tools  {}",
        match (&config.api_url, config.mcp_servers.is_empty()) {
            (Some(url), false) => format!("{}/api/mcp", url),
            _ => "off".to_string(),
        }
    );
    eprintln!();
    let page_url = format!("http://127.0.0.1:{}/?secret={}", config.port, config.secret);
    eprintln!("  Open: {}", page_url);
    eprintln!();

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", config.port))
        .await
        .map_err(|e| format!("port {}: {} (is Builder already running?)", config.port, e))?;
    if let Some(path) = &cli.url_file {
        write_private(path, &page_url).map_err(|e| format!("{}: {}", path.display(), e))?;
    }
    if cli.open {
        open_in_browser(&page_url);
    }

    let url_file = cli.url_file.clone();
    let result = builder::serve_on(listener, config, shutdown_signal()).await;
    if let Some(path) = url_file {
        let _ = std::fs::remove_file(path);
    }
    result
}
