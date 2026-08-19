//! Tauri application: persist settings, spawn or connect to dsh web, and update DSH.

mod config;
mod dsh;

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;
use url::Url;

use config::{parse_ready_line, DshConfig, LaunchMode};
use dsh::{
    can_launch_local, install_managed, launch_command_line, managed_version, needs_managed_install,
    registry_version, resolve_launch, runtime_prefix, scan_lines, spawn_plan, take_pipes,
    wrap_child, DshProcess,
};

const STORE_FILE: &str = "desktop.json";
const STORE_KEY: &str = "config";

/// Live backend snapshot published to the settings page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    state: StatusState,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    config: DshConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_version: Option<String>,
    can_launch_local: bool,
    platform: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum StatusState {
    Idle,
    Installing,
    Starting,
    Ready,
    Updating,
    Error,
}

struct AppState {
    config: Mutex<DshConfig>,
    process: Mutex<Option<DshProcess>>,
    url: Mutex<Option<String>>,
    state: Mutex<StatusState>,
    message: Mutex<Option<String>>,
    installed: Mutex<Option<String>>,
    latest: Mutex<Option<String>>,
    shell_url: Mutex<String>,
}

impl AppState {
    fn new(config: DshConfig) -> Self {
        Self {
            config: Mutex::new(config),
            process: Mutex::new(None),
            url: Mutex::new(None),
            state: Mutex::new(StatusState::Idle),
            message: Mutex::new(None),
            installed: Mutex::new(None),
            latest: Mutex::new(None),
            shell_url: Mutex::new(String::new()),
        }
    }

    async fn snapshot(&self) -> BackendStatus {
        BackendStatus {
            state: *self.state.lock().await,
            url: self.url.lock().await.clone(),
            message: self.message.lock().await.clone(),
            config: self.config.lock().await.clone(),
            installed_version: self.installed.lock().await.clone(),
            latest_version: self.latest.lock().await.clone(),
            can_launch_local: can_launch_local(),
            platform: std::env::consts::OS.to_string(),
        }
    }
}

fn load_config(app: &AppHandle) -> DshConfig {
    let Ok(store) = app.store(STORE_FILE) else {
        return default_config_for_platform();
    };
    let mut config = match store.get(STORE_KEY) {
        Some(value) => serde_json::from_value(value).unwrap_or_default(),
        None => DshConfig::default(),
    };
    if !can_launch_local() {
        config.launch_mode = LaunchMode::Connect;
    }
    config
}

fn default_config_for_platform() -> DshConfig {
    let mut config = DshConfig::default();
    if !can_launch_local() {
        config.launch_mode = LaunchMode::Connect;
    }
    config
}

fn save_config(app: &AppHandle, config: &DshConfig) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|error| error.to_string())?;
    store.set(STORE_KEY, serde_json::to_value(config).map_err(|error| error.to_string())?);
    store.save().map_err(|error| error.to_string())
}

async fn publish(app: &AppHandle, state: &AppState) {
    let snapshot = state.snapshot().await;
    let _ = app.emit("dsh-status", snapshot);
}

async fn set_state(app: &AppHandle, state: &AppState, next: StatusState, message: Option<String>) {
    *state.state.lock().await = next;
    *state.message.lock().await = message;
    publish(app, state).await;
}

fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window missing".to_string())
}

fn navigate(app: &AppHandle, url: &str) -> Result<(), String> {
    let window = main_window(app)?;
    let parsed = Url::parse(url).map_err(|error| error.to_string())?;
    window.navigate(parsed).map_err(|error| error.to_string())
}

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_status(state: State<'_, Arc<AppState>>) -> Result<BackendStatus, String> {
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn set_config(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    config: DshConfig,
) -> Result<DshConfig, String> {
    config.validate()?;
    save_config(&app, &config)?;
    *state.config.lock().await = config.clone();
    publish(&app, &state).await;
    Ok(config)
}

#[tauri::command]
async fn start_dsh(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<BackendStatus, String> {
    let config = state.config.lock().await.clone();
    config.validate()?;
    if !can_launch_local() && config.launch_mode == LaunchMode::Local {
        return Err("this platform cannot spawn dsh web; switch to connect mode".into());
    }
    match config.launch_mode {
        LaunchMode::Connect => connect_existing(&app, &state, config).await,
        LaunchMode::Local => spawn_local(&app, &state, config).await,
    }
}

async fn connect_existing(
    app: &AppHandle,
    state: &AppState,
    config: DshConfig,
) -> Result<BackendStatus, String> {
    set_state(app, state, StatusState::Starting, Some("Connecting".into())).await;
    let url = config.web_url()?;
    wait_for_http(&url).await?;
    *state.url.lock().await = Some(url.clone());
    set_state(app, state, StatusState::Ready, Some("Connected".into())).await;
    let _ = navigate(app, &url);
    Ok(state.snapshot().await)
}

async fn spawn_local(
    app: &AppHandle,
    state: &AppState,
    config: DshConfig,
) -> Result<BackendStatus, String> {
    stop_inner(state).await?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let data = app_data_dir(app)?;
    if needs_managed_install(&data) {
        set_state(
            app,
            state,
            StatusState::Installing,
            Some(format!("Installing {} into the app runtime prefix", dsh::DSH_PACKAGE)),
        )
        .await;
        let handle = app.clone();
        let prefix = runtime_prefix(&data);
        if let Err(error) = install_managed(&prefix, dsh::DSH_PACKAGE_SPEC, move |line| {
            let _ = handle.emit("dsh-spawn-log", line);
        })
        .await
        {
            set_state(app, state, StatusState::Error, Some(error.clone())).await;
            return Err(error);
        }
        *state.installed.lock().await = managed_version(&runtime_prefix(&data));
    }
    set_state(app, state, StatusState::Starting, Some("Spawning dsh web".into())).await;
    let plan = resolve_launch(&data, &cwd, &config)?;
    let _ = app.emit("dsh-spawn-log", format!("$ {}", launch_command_line(&plan)));
    let child = spawn_plan(&plan)?;
    let mut process = wrap_child(child);
    let (stdout, stderr) = take_pipes(&mut process)?;
    *state.process.lock().await = Some(process);

    let found = Arc::new(Mutex::new(None::<String>));
    let logs = Arc::new(Mutex::new(Vec::<String>::new()));
    let found_out = found.clone();
    let found_err = found.clone();
    let logs_out = logs.clone();
    let logs_err = logs.clone();
    let emit_out = app.clone();
    let emit_err = app.clone();
    let scan_out = tokio::spawn(async move {
        let _ = scan_lines(stdout, |line| {
            let _ = emit_out.emit("dsh-spawn-log", line.to_string());
            if let Ok(mut slot) = logs_out.try_lock() {
                slot.push(line.to_string());
            }
            if let Some(url) = parse_ready_line(line) {
                if let Ok(mut slot) = found_out.try_lock() {
                    *slot = Some(url);
                    return true;
                }
            }
            false
        })
        .await;
    });
    let scan_err = tokio::spawn(async move {
        let _ = scan_lines(stderr, |line| {
            let _ = emit_err.emit("dsh-spawn-log", line.to_string());
            if let Ok(mut slot) = logs_err.try_lock() {
                slot.push(line.to_string());
            }
            if let Some(url) = parse_ready_line(line) {
                if let Ok(mut slot) = found_err.try_lock() {
                    *slot = Some(url);
                    return true;
                }
            }
            false
        })
        .await;
    });

    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let url = loop {
        if let Some(url) = found.lock().await.clone() {
            break url;
        }
        if let Some(process) = state.process.lock().await.as_mut() {
            if let Ok(Some(status)) = process.try_wait() {
                scan_out.abort();
                scan_err.abort();
                let tail = tail_logs(&logs).await;
                let message = format!("dsh web exited ({status}) before it was ready.{tail}");
                set_state(app, state, StatusState::Error, Some(message.clone())).await;
                return Err(message);
            }
        }
        if config.port != 0 {
            let candidate = config.web_url()?;
            if probe_http(&candidate).await {
                break candidate;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            scan_out.abort();
            scan_err.abort();
            stop_inner(state).await?;
            let tail = tail_logs(&logs).await;
            let message = format!("dsh web did not become ready within 90s.{tail}");
            set_state(app, state, StatusState::Error, Some(message.clone())).await;
            return Err(message);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    *state.url.lock().await = Some(url.clone());
    set_state(app, state, StatusState::Ready, Some("dsh web is listening".into())).await;
    let _ = navigate(app, &url);
    Ok(state.snapshot().await)
}

async fn tail_logs(logs: &Mutex<Vec<String>>) -> String {
    let lines = logs.lock().await;
    if lines.is_empty() {
        return String::new();
    }
    let start = lines.len().saturating_sub(12);
    format!("\n{}", lines[start..].join("\n"))
}

async fn wait_for_http(url: &str) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if probe_http(url).await {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no HTTP server at {url}"));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn probe_http(url: &str) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let port = url.port_or_known_default().unwrap_or(80);
    tokio::time::timeout(
        Duration::from_millis(400),
        tokio::net::TcpStream::connect((host, port)),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .is_some()
}

async fn stop_inner(state: &AppState) -> Result<(), String> {
    if let Some(mut child) = state.process.lock().await.take() {
        child.stop().await?;
    }
    *state.url.lock().await = None;
    Ok(())
}

#[tauri::command]
async fn stop_dsh(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<BackendStatus, String> {
    stop_inner(&state).await?;
    set_state(&app, &state, StatusState::Idle, Some("Stopped".into())).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn open_web(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let url = state
        .url
        .lock()
        .await
        .clone()
        .ok_or_else(|| "dsh web is not ready".to_string())?;
    navigate(&app, &url)
}

#[tauri::command]
async fn open_settings(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let url = state.shell_url.lock().await.clone();
    if url.is_empty() {
        return Err("shell URL is not available".into());
    }
    navigate(&app, &url)
}

#[tauri::command]
async fn check_dsh_updates(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<BackendStatus, String> {
    let data = app_data_dir(&app)?;
    *state.installed.lock().await = managed_version(&runtime_prefix(&data));
    match registry_version().await {
        Ok(latest) => *state.latest.lock().await = Some(latest),
        Err(error) => {
            set_state(&app, &state, StatusState::Error, Some(error.clone())).await;
            return Err(error);
        }
    }
    publish(&app, &state).await;
    Ok(state.snapshot().await)
}

#[tauri::command]
async fn update_dsh(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<BackendStatus, String> {
    if !can_launch_local() {
        return Err("DSH updates require a desktop Node/npm install".into());
    }
    set_state(&app, &state, StatusState::Updating, Some("Updating DSH".into())).await;
    let data = app_data_dir(&app)?;
    let prefix = runtime_prefix(&data);
    let handle = app.clone();
    let result = install_managed(&prefix, &format!("{}@latest", dsh::DSH_PACKAGE), move |line| {
        let _ = handle.emit("dsh-update-progress", line);
    })
    .await;
    match result {
        Ok(()) => {
            *state.installed.lock().await = managed_version(&prefix);
            set_state(
                &app,
                &state,
                StatusState::Idle,
                Some("DSH update complete; start again to use it".into()),
            )
            .await;
            Ok(state.snapshot().await)
        }
        Err(error) => {
            set_state(&app, &state, StatusState::Error, Some(error.clone())).await;
            Err(error)
        }
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let reload = MenuItemBuilder::with_id("open-web", "Open Web UI").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&settings)
        .item(&reload)
        .separator()
        .item(&quit)
        .build()?;
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let state = state.inner().clone();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let url = state.shell_url.lock().await.clone();
                        if !url.is_empty() {
                            let _ = navigate(&app, &url);
                        }
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    });
                }
            }
            "open-web" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let state = state.inner().clone();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(url) = state.url.lock().await.clone() {
                            let _ = navigate(&app, &url);
                        }
                    });
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn build_tray(_app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

/// Application entry used by both the desktop binary and the mobile library.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_store::Builder::new().build());
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }
    builder
        .setup(|app| {
            let config = load_config(app.handle());
            let state = Arc::new(AppState::new(config));
            if let Ok(data) = app_data_dir(app.handle()) {
                if let Some(version) = managed_version(&runtime_prefix(&data)) {
                    if let Ok(mut installed) = state.installed.try_lock() {
                        *installed = Some(version);
                    }
                }
            }
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(url) = window.url() {
                    if let Ok(mut slot) = state.shell_url.try_lock() {
                        *slot = url.to_string();
                    }
                }
            }
            app.manage(state);
            if let Err(error) = build_tray(app.handle()) {
                eprintln!("dsh-desktop: tray unavailable: {error}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_config,
            start_dsh,
            stop_dsh,
            open_web,
            open_settings,
            check_dsh_updates,
            update_dsh,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<Arc<AppState>>() {
                    tauri::async_runtime::block_on(async {
                        let _ = stop_inner(&state).await;
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running dsh-desktop");
}
