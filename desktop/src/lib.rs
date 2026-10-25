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
    api_login: std::sync::Mutex<Option<(String, String)>>,
}

#[tauri::command]
fn onboarding_info() -> Result<Settings, String> { Settings::load() }

#[tauri::command]
async fn onboarding_folder(app: AppHandle, api: bool) -> Option<String> {
    pick_workspace(&app, if api { "Choose the voipappz-api checkout" } else { "Choose the React app Git checkout (voipappz/app)" })
        .await.map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
async fn onboarding_check(api_url: String, api_dir: String, email: String, password: String) -> Result<String, String> {
    let dir = (!api_dir.trim().is_empty()).then(|| PathBuf::from(api_dir.trim()));
    let url = launcher::onboarding_api_url(&api_url, dir.as_deref())?;
    launcher::check_api_connection(&url, &email, &password).await
}

#[tauri::command]
async fn onboarding_start(app: AppHandle, app_repo: String, api_url: String, api_dir: String, email: String, password: String) -> Result<(), String> {
    let settings = launcher::onboarding_settings(Settings::load()?, &app_repo, &api_url, &api_dir)?;
    let url = settings.api_url.clone().ok_or("Choose an API address.")?;
    launcher::check_api_connection(&url, &email, &password).await?;
    let basic = if email.is_empty() && password.is_empty() { None }
        else { Some((url, launcher::api_credential(&email, &password)?)) };
    {
        let state = app.state::<AppState>();
        let _guard = state.running.try_lock().map_err(|_| "Setup is already running. Follow its progress below.")?;
        settings.save()?;
        *state.api_login.lock().unwrap_or_else(|e| e.into_inner()) = basic;
    }
    tauri::async_runtime::spawn(restart(app));
    Ok(())
}

fn show_onboarding(app: &AppHandle) {
    show_startup(app);
    progress(app, "Connect your React app project and the API to get started.", "configure");
}

#[derive(Clone, Default, serde::Serialize)]
struct StartupProgress {
    status: String,
    /// The setup step in progress (an id from `launcher::SETUP_STEPS`).
    step: String,
    steps: Vec<(String, String)>,
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
    let changed = {
        let mut startup = state.startup.lock().unwrap_or_else(|e| e.into_inner());
        let changed = startup.status != status;
        startup.status = status.into();
        match line.strip_prefix(launcher::STEP_PREFIX) {
            Some(id) => {
                startup.step = id.into();
                startup.lines.push(format!("▸ {}", launcher::step_label(id).unwrap_or(id)));
            }
            None => startup.lines.push(line.into()),
        }
        if startup.lines.len() > 300 { startup.lines.remove(0); }
        changed
    };
    if changed {
        set_tray_status(app, status);
    }
}

const TRAY_ID: &str = "main";

/// Show the state in the tray: the app icon with a coloured dot (yellow
/// while setting up, green when ready, red on failure, grey before setup).
fn set_tray_status(app: &AppHandle, status: &str) {
    let (colour, tooltip) = match status {
        "ready" => ([46, 204, 64], "VoipAppz Builder — running"),
        "failed" => ([229, 57, 53], "VoipAppz Builder — setup failed"),
        "configure" => ([150, 150, 150], "VoipAppz Builder — not connected"),
        _ => ([255, 196, 0], "VoipAppz Builder — setting up…"),
    };
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let _ = tray.set_tooltip(Some(tooltip));
    if let Some(icon) = app.default_window_icon().map(|base| status_icon(base, colour)) {
        let _ = tray.set_icon(Some(icon));
    }
}

/// The base icon with a filled, dark-edged dot in the bottom-right corner.
fn status_icon(base: &tauri::image::Image<'_>, colour: [u8; 3]) -> tauri::image::Image<'static> {
    let (width, height) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();
    let radius = (width.min(height) as f32 * 0.24).max(3.0);
    let (cx, cy) = (width as f32 - radius - 1.0, height as f32 - radius - 1.0);
    for y in 0..height {
        for x in 0..width {
            let distance = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            if distance > radius { continue; }
            let i = ((y * width + x) * 4) as usize;
            let pixel = if distance > radius - radius.max(4.0) / 4.0 { [17, 17, 17, 255] } else { [colour[0], colour[1], colour[2], 255] };
            rgba[i..i + 4].copy_from_slice(&pixel);
        }
    }
    tauri::image::Image::new_owned(rgba, width, height)
}

fn show_startup(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        *state.startup.lock().unwrap_or_else(|e| e.into_inner()) = StartupProgress {
            status: "working".into(), step: String::new(),
            steps: launcher::SETUP_STEPS.iter().map(|(id, label)| (id.to_string(), label.to_string())).collect(),
            lines: vec!["Starting VoipAppz Builder…".into()],
            log_path: log_path().to_string_lossy().into_owned(),
        };
    }
    set_tray_status(app, "working");
    if let Some(window) = app.get_webview_window("startup") {
        let _ = window.show();
        let _ = window.set_focus();
    } else if let Err(error) = tauri::WebviewWindowBuilder::new(app, "startup", tauri::WebviewUrl::App("index.html".into()))
        .title("VoipAppz Builder — Startup")
        .inner_size(780.0, 800.0)
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

async fn pick_workspace(app: &AppHandle, title: &str) -> Option<PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
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

    let settings = match Settings::load() {
        Ok(s) => s,
        Err(e) => return show_error(&app, format!("Settings could not be read: {}", e)),
    };
    if settings.workspace.is_none() && settings.app_repo.is_none() {
        progress(&app, "Choose your React app folder and API address above.", "configure");
        return;
    }
    let basic = state.api_login.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
        .filter(|(url, _)| settings.api_url.as_ref() == Some(url)).map(|(_, basic)| basic.clone());
    match launcher::start_with_api_credential(&settings, basic, |line| progress(&app, line, "working")).await {
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

pub fn run() {
    let _log_guard = init_logging();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![startup_progress, onboarding_info, onboarding_folder, onboarding_check, onboarding_start])
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
            let workspace = MenuItem::with_id(app, "workspace", "Connect App and API…", true, None::<&str>)?;
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

            let mut tray = TrayIconBuilder::with_id(TRAY_ID)
                .tooltip("VoipAppz Builder")
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    let app = app.clone();
                    match event.id.as_ref() {
                        "open" => {
                            tauri::async_runtime::spawn(open_page(app));
                        }
                        "workspace" => {
                            show_onboarding(&app);
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

            show_onboarding(app.handle());
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
