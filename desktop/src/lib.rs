//! VoipAppz Builder desktop app: a tray icon that runs the Builder server
//! (core::launcher), shows setup progress, and opens it in the browser.

use builder_core::launcher::{self, Running, Settings};
use std::path::PathBuf;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tokio::sync::Mutex;
use tracing::{error, info};

#[derive(Default)]
struct AppState {
    running: Mutex<Option<Running>>,
    startup: std::sync::Mutex<StartupProgress>,
}

#[derive(Clone, Default, serde::Serialize)]
struct StartupProgress {
    status: String,
    lines: Vec<String>,
    log_path: String,
}

#[tauri::command]
fn startup_progress(state: tauri::State<'_, AppState>) -> StartupProgress {
    state.startup.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn progress(app: &AppHandle, line: &str, status: &str) {
    info!("{}", line);
    let state = app.state::<AppState>();
    let mut startup = state.startup.lock().unwrap_or_else(|e| e.into_inner());
    startup.status = status.into();
    startup.lines.push(line.into());
    if startup.lines.len() > 300 { startup.lines.remove(0); }
}

fn show_startup(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        *state.startup.lock().unwrap_or_else(|e| e.into_inner()) = StartupProgress {
            status: "working".into(), lines: vec!["Starting VoipAppz Builder…".into()],
            log_path: log_path().to_string_lossy().into_owned(),
        };
    }
    if let Some(window) = app.get_webview_window("startup") {
        let _ = window.show();
        let _ = window.set_focus();
    } else if let Err(error) = tauri::WebviewWindowBuilder::new(app, "startup", tauri::WebviewUrl::App("index.html".into()))
        .title("VoipAppz Builder — Startup")
        .inner_size(720.0, 560.0)
        .min_inner_size(420.0, 360.0)
        .build() {
        error!("Could not show setup progress: {}", error);
    }
}

fn log_path() -> PathBuf {
    launcher::state_dir().join("builder.log")
}

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dir = launcher::state_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let appender = tracing_appender::rolling::never(&dir, "builder.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter("info")
        .init();
    Some(guard)
}

fn open_target(app: &AppHandle, target: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        // Inside an AppImage, child processes must not inherit its library paths.
        let mut command = std::process::Command::new("xdg-open");
        command.arg(target);
        builder_core::cleanup_appimage_env(&mut command);
        if let Err(e) = command.spawn() {
            error!("Could not open {}: {}", target, e);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_opener::OpenerExt;
        if let Err(e) = app.opener().open_url(target, None::<&str>) {
            error!("Could not open {}: {}", target, e);
        }
    }
}

fn show_error(app: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    progress(app, &message, "failed");
    app.dialog()
        .message(message)
        .title("VoipAppz Builder")
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}

async fn pick_workspace(app: &AppHandle) -> Option<PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose the VoipAppz app repository (../app); Builder makes its own worktree of it")
        .pick_folder(move |folder| {
            let _ = tx.send(folder.and_then(|f| f.into_path().ok()));
        });
    rx.await.ok().flatten()
}

/// Stop what runs, then start from the saved settings and open the page.
async fn restart(app: AppHandle) {
    let state = app.state::<AppState>();
    let Ok(mut running) = state.running.try_lock() else {
        if let Some(window) = app.get_webview_window("startup") {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return;
    };
    show_startup(&app);
    if let Some(current) = running.take() {
        current.stop().await;
    }

    let mut settings = match Settings::load() {
        Ok(s) => s,
        Err(e) => return show_error(&app, format!("Settings could not be read: {}", e)),
    };
    if settings.workspace.is_none() && settings.app_repo.is_none() {
        progress(&app, "Choose the app repository in the folder selection dialog.", "working");
        match pick_workspace(&app).await {
            Some(folder) => {
                set_app_repo(&mut settings, folder);
                if let Err(e) = settings.save() {
                    return show_error(&app, format!("Settings could not be saved: {}", e));
                }
            }
            None => {
                return show_error(
                    &app,
                    "No app repository chosen. Use \"Choose App Repository…\" in the VoipAppz Builder menu.",
                )
            }
        }
    }

    match launcher::start(&settings, |line| progress(&app, line, "working")).await {
        Ok(started) => {
            info!("Builder is running; {}", started.api_note);
            progress(&app, "Builder is ready. Opening your browser. You can close this setup window.", "ready");
            open_target(&app, &started.url);
            *running = Some(started);
        }
        Err(e) => {
            error!("Builder could not start: {}", e);
            show_error(&app, format!("VoipAppz Builder could not start.\n\n{}", e));
        }
    }
}

async fn open_page(app: AppHandle) {
    let state = app.state::<AppState>();
    let url = match state.running.try_lock() {
        Ok(running) => running.as_ref().map(|r| r.url.clone()),
        Err(_) => {
            if let Some(window) = app.get_webview_window("startup") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return;
        }
    };
    match url {
        Some(url) => open_target(&app, &url),
        None => restart(app).await,
    }
}

/// Use `folder` as the app repository, Builder's own worktree of it as the
/// workspace, and a sibling voipappz-api checkout for the API, if present.
fn set_app_repo(settings: &mut Settings, folder: PathBuf) {
    if settings.api_dir.is_none() {
        let api = folder.with_file_name("voipappz-api");
        if api.join("bin/dev-status.sh").exists() {
            settings.api_dir = Some(api);
        }
    }
    settings.app_repo = Some(folder);
    settings.workspace = None;
}

async fn choose_workspace(app: AppHandle) {
    if let Some(folder) = pick_workspace(&app).await {
        let mut settings = Settings::load().unwrap_or_default();
        set_app_repo(&mut settings, folder);
        if let Err(e) = settings.save() {
            return show_error(&app, format!("Settings could not be saved: {}", e));
        }
        restart(app).await;
    }
}

pub fn run() {
    let _log_guard = init_logging();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![startup_progress])
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tauri::async_runtime::spawn(open_page(app.clone()));
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let open = MenuItem::with_id(app, "open", "Open VoipAppz Builder", true, None::<&str>)?;
            let workspace = MenuItem::with_id(app, "workspace", "Choose App Repository…", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let logs = MenuItem::with_id(app, "logs", "View Logs", true, None::<&str>)?;
            let at_login = CheckMenuItem::with_id(
                app,
                "at_login",
                "Launch at Login",
                true,
                app.autolaunch().is_enabled().unwrap_or(false),
                None::<&str>,
            )?;
            let restart_item = MenuItem::with_id(app, "restart", "Restart", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit VoipAppz Builder", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let separator2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[&open, &separator, &workspace, &settings, &logs, &separator2, &at_login, &restart_item, &quit],
            )?;

            let mut tray = TrayIconBuilder::new()
                .tooltip("VoipAppz Builder")
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    let app = app.clone();
                    match event.id.as_ref() {
                        "open" => {
                            tauri::async_runtime::spawn(open_page(app));
                        }
                        "workspace" => {
                            tauri::async_runtime::spawn(choose_workspace(app));
                        }
                        "settings" => {
                            if !Settings::path().exists() {
                                let _ = Settings::load().unwrap_or_default().save();
                            }
                            open_target(&app, &Settings::path().to_string_lossy());
                        }
                        "logs" => open_target(&app, &log_path().to_string_lossy()),
                        "at_login" => {
                            let autolaunch = app.autolaunch();
                            let enabled = autolaunch.is_enabled().unwrap_or(false);
                            let result = if enabled { autolaunch.disable() } else { autolaunch.enable() };
                            if let Err(e) = result {
                                error!("Launch at login could not be changed: {}", e);
                            }
                        }
                        "restart" => {
                            tauri::async_runtime::spawn(restart(app));
                        }
                        "quit" => {
                            tauri::async_runtime::spawn(async move {
                                let state = app.state::<AppState>();
                                let running = state.running.try_lock().ok().and_then(|mut running| running.take());
                                if let Some(running) = running {
                                    running.stop().await;
                                }
                                app.exit(0);
                            });
                        }
                        _ => {}
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            tauri::async_runtime::spawn(restart(app.handle().clone()));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building VoipAppz Builder")
        .run(|_app, event| {
            // No windows: keep running in the tray until Quit.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
