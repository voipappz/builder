//! VoipAppz Builder desktop app: a tray icon that runs the Builder server
//! (core::launcher) and opens it in the browser. It has no window of its own.

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
        .set_title("Choose the app checkout VoipAppz Builder works in")
        .pick_folder(move |folder| {
            let _ = tx.send(folder.and_then(|f| f.into_path().ok()));
        });
    rx.await.ok().flatten()
}

/// Stop what runs, then start from the saved settings and open the page.
async fn restart(app: AppHandle) {
    let state = app.state::<AppState>();
    let mut running = state.running.lock().await;
    if let Some(current) = running.take() {
        current.stop().await;
    }

    let mut settings = match Settings::load() {
        Ok(s) => s,
        Err(e) => return show_error(&app, format!("Settings could not be read: {}", e)),
    };
    if settings.workspace.is_none() {
        match pick_workspace(&app).await {
            Some(folder) => {
                settings.workspace = Some(folder);
                if let Err(e) = settings.save() {
                    return show_error(&app, format!("Settings could not be saved: {}", e));
                }
            }
            None => {
                return show_error(
                    &app,
                    "No workspace chosen. Use \"Choose Workspace…\" in the VoipAppz Builder menu.",
                )
            }
        }
    }

    match launcher::start(&settings, |line| info!("{}", line)).await {
        Ok(started) => {
            info!("Builder is running; {}", started.api_note);
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
    let url = app
        .state::<AppState>()
        .running
        .lock()
        .await
        .as_ref()
        .map(|r| r.url.clone());
    match url {
        Some(url) => open_target(&app, &url),
        None => restart(app).await,
    }
}

async fn choose_workspace(app: AppHandle) {
    if let Some(folder) = pick_workspace(&app).await {
        let mut settings = Settings::load().unwrap_or_default();
        settings.workspace = Some(folder);
        if let Err(e) = settings.save() {
            return show_error(&app, format!("Settings could not be saved: {}", e));
        }
        restart(app).await;
    }
}

pub fn run() {
    let _log_guard = init_logging();

    tauri::Builder::default()
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
            let workspace = MenuItem::with_id(app, "workspace", "Choose Workspace…", true, None::<&str>)?;
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
                                if let Some(running) = app.state::<AppState>().running.lock().await.take() {
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
