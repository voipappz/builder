use builder_core::launcher::{self, Settings, DEFAULT_AGENT_COMMAND};
use clap::Parser;
use std::path::PathBuf;

/// VoipAppz Builder: chat with a coding agent about the VoipAppz app and see
/// it running.
#[derive(Parser, Debug)]
#[command(name = "voipappz-builder", version, about)]
struct Cli {
    /// The app repository (../app). Builder works in its own git worktree of
    /// it (<app-repo>-builder, branch builder/work from origin/main),
    /// created on first start.
    #[arg(long, required_unless_present = "workspace")]
    app_repo: Option<PathBuf>,

    /// Work in this checkout instead of Builder's own worktree.
    #[arg(long)]
    workspace: Option<PathBuf>,

    #[arg(short, long, default_value_t = 9832)]
    port: u16,

    /// Where the chat client is loaded from: the app's dev server, or a
    /// deployed app.
    #[arg(long, env = "VOIPAPPZ_BUILDER_CLIENT_URL", default_value = "http://localhost:3000")]
    client_url: String,

    /// The app shown in the Admin and User previews (default: --client-url).
    #[arg(long, env = "VOIPAPPZ_BUILDER_PREVIEW_URL")]
    preview_url: Option<String>,

    /// Start `npm run dev` in the workspace when the client does not answer.
    #[arg(long)]
    start_app: bool,

    /// VoipAppz API whose /api/mcp tools the agent gets
    /// (default: http://127.0.0.1:<VA_WEB_PORT> from --api-dir's .env).
    #[arg(long, env = "VOIPAPPZ_API_URL")]
    api_url: Option<String>,

    /// The voipappz-api checkout; gives the API address and, when
    /// VA_MCP_BASIC is unset, the login (MCP_AUTH=1 bin/dev-status.sh).
    #[arg(long)]
    api_dir: Option<PathBuf>,

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

fn absolute(path: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
    path.map(|p| std::path::absolute(&p).map_err(|e| format!("{}: {}", p.display(), e)))
        .transpose()
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

    let settings = Settings {
        app_repo: absolute(cli.app_repo)?,
        workspace: absolute(cli.workspace)?,
        client_url: cli.client_url,
        preview_url: cli.preview_url,
        api_url: cli.api_url,
        api_dir: absolute(cli.api_dir)?,
        start_app: cli.start_app,
        port: cli.port,
        agent_command: cli.agent_command,
    };

    let running = launcher::start(&settings, |line| match line.strip_prefix(launcher::STEP_PREFIX) {
        Some(id) => eprintln!("\n== {}", launcher::step_label(id).unwrap_or(id)),
        None => eprintln!("  {}", line),
    }).await?;

    eprintln!();
    eprintln!("VoipAppz Builder");
    eprintln!("  workspace  {}", running.workspace.display());
    eprintln!("  {}", running.api_note);
    eprintln!("  Open: {}", running.url);
    eprintln!();

    if let Some(path) = &cli.url_file {
        write_private(path, &running.url).map_err(|e| format!("{}: {}", path.display(), e))?;
    }
    if cli.open {
        open_in_browser(&running.url);
    }

    shutdown_signal().await;
    running.stop().await;
    if let Some(path) = &cli.url_file {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}
