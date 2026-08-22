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

use config::{boot_manifest_ready, parse_ready_line, DshConfig, LaunchMode};
use dsh::{
    can_launch_local, install_managed, launch_command_line, managed_version, needs_managed_install,
    plugin_add_args, plugin_already_present, registry_version, resolve_cli, resolve_launch,
    run_plan, runtime_prefix, scan_lines, spawn_plan, take_pipes, wrap_child, DshProcess,
    STARTER_PLUGINS,
};

const STORE_FILE: &str = "desktop.json";
const STORE_KEY: &str = "config";
const PLUGINS_KEY: &str = "starterPlugins";
const DSH_WEBVIEW: &str = "dsh";
const TITLEBAR_LOGICAL: f64 = 40.0;

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

fn load_starter_plugins(app: &AppHandle) -> Vec<String> {
    let Ok(store) = app.store(STORE_FILE) else {
        return Vec::new();
    };
    match store.get(PLUGINS_KEY) {
        Some(value) => serde_json::from_value(value).unwrap_or_default(),
        None => Vec::new(),
    }
}

fn save_starter_plugins(app: &AppHandle, specs: &[String]) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|error| error.to_string())?;
    store.set(
        PLUGINS_KEY,
        serde_json::to_value(specs).map_err(|error| error.to_string())?,
    );
    store.save().map_err(|error| error.to_string())
}

#[derive(Clone, Serialize)]
struct ThemePayload {
    bg: String,
    fg: String,
}

fn reveal_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn layout_dsh_webview(
    size: tauri::PhysicalSize<u32>,
    scale: f64,
    webview: &tauri::Webview,
) -> Result<(), String> {
    let top = (TITLEBAR_LOGICAL * scale).round() as u32;
    let height = size.height.saturating_sub(top).max(1);
    webview
        .set_position(tauri::PhysicalPosition::new(0, top))
        .map_err(|error| error.to_string())?;
    webview
        .set_size(tauri::PhysicalSize::new(size.width, height))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn hide_dsh_ui(app: &AppHandle) {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        if let Some(webview) = app.get_webview(DSH_WEBVIEW) {
            let _ = webview.hide();
        }
    }
}

fn show_dsh_ui(app: &AppHandle, url: &str) -> Result<(), String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return navigate(app, url);
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let parsed = Url::parse(url).map_err(|error| error.to_string())?;
        let window = app
            .get_window("main")
            .ok_or_else(|| "main window missing".to_string())?;
        let scale = window.scale_factor().map_err(|error| error.to_string())?;
        let size = window.inner_size().map_err(|error| error.to_string())?;
        if let Some(existing) = app.get_webview(DSH_WEBVIEW) {
            existing.navigate(parsed).map_err(|error| error.to_string())?;
            layout_dsh_webview(size, scale, &existing)?;
            existing.show().map_err(|error| error.to_string())?;
            let _ = existing.set_focus();
            let _ = existing.eval(include_str!("disable_context_menu.js"));
            return Ok(());
        }
        let logical_w = f64::from(size.width) / scale;
        let logical_h = f64::from(size.height) / scale;
        let builder = tauri::webview::WebviewBuilder::new(
            DSH_WEBVIEW,
            tauri::WebviewUrl::External(parsed),
        )
        .initialization_script(include_str!("disable_context_menu.js"))
        .initialization_script(include_str!("theme_bridge.js"));
        match window.add_child(
            builder,
            tauri::LogicalPosition::new(0.0, TITLEBAR_LOGICAL),
            tauri::LogicalSize::new(logical_w, (logical_h - TITLEBAR_LOGICAL).max(1.0)),
        ) {
            Ok(webview) => {
                let _ = webview.set_focus();
                let _ = webview.eval(include_str!("disable_context_menu.js"));
                Ok(())
            }
            Err(error) => {
                eprintln!("dsh-desktop: child webview unavailable: {error}");
                navigate(app, url)
            }
        }
    }
}

async fn install_starter_plugins(
    app: &AppHandle,
    state: &AppState,
    data: &std::path::Path,
    hints: &[std::path::PathBuf],
) -> Result<(), String> {
    let mut done = load_starter_plugins(app);
    let total = STARTER_PLUGINS.len();
    for (index, (name, spec)) in STARTER_PLUGINS.iter().enumerate() {
        if done.iter().any(|installed| installed == spec) {
            continue;
        }
        let n = index + 1;
        set_state(
            app,
            state,
            StatusState::Installing,
            Some(format!("Installing plugin {n}/{total}: {name}")),
        )
        .await;
        let _ = app.emit(
            "dsh-spawn-log",
            format!("$ dsh plugin --profile web add {spec}"),
        );
        let plan = resolve_cli(data, hints, &plugin_add_args(spec))?;
        let handle = app.clone();
        let result = tokio::time::timeout(
            Duration::from_secs(180),
            run_plan(&plan, move |line| {
                let _ = handle.emit("dsh-spawn-log", line);
            }),
        )
        .await;
        match result {
            Ok(Ok(())) => {
                done.push((*spec).to_string());
                let _ = save_starter_plugins(app, &done);
            }
            Ok(Err(error)) => {
                let _ = app.emit("dsh-spawn-log", format!("plugin {name}: {error}"));
                if plugin_already_present(&error) {
                    done.push((*spec).to_string());
                    let _ = save_starter_plugins(app, &done);
                }
            }
            Err(_) => {
                let _ = app.emit("dsh-spawn-log", format!("plugin {name}: timed out"));
            }
        }
    }
    Ok(())
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

const NO_CONTEXT_MENU_JS: &str = include_str!("disable_context_menu.js");

fn inject_no_context_menu(window: &WebviewWindow) {
    let _ = window.eval(NO_CONTEXT_MENU_JS);
    let win = window.clone();
    tauri::async_runtime::spawn(async move {
        for delay in [80_u64, 200, 500, 1200] {
            tokio::time::sleep(Duration::from_millis(delay)).await;
            let _ = win.eval(NO_CONTEXT_MENU_JS);
        }
    });
}

#[allow(dead_code)]
fn navigate(app: &AppHandle, url: &str) -> Result<(), String> {
    let window = main_window(app)?;
    let parsed = Url::parse(url).map_err(|error| error.to_string())?;
    window.navigate(parsed).map_err(|error| error.to_string())?;
    inject_no_context_menu(&window);
    Ok(())
}

#[allow(dead_code)]
fn settings_page_url(base: &str) -> Result<String, String> {
    let mut parsed = Url::parse(base).map_err(|error| error.to_string())?;
    parsed.set_query(Some("settings=1"));
    Ok(parsed.to_string())
}

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| error.to_string())
}

fn runtime_hints(app: &AppHandle) -> Vec<std::path::PathBuf> {
    let mut hints = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        hints.push(dir);
    }
    if let Ok(dir) = std::env::current_dir() {
        hints.push(dir);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            hints.push(dir.to_path_buf());
        }
    }
    hints
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

async fn start_inner(app: &AppHandle, state: &AppState) -> Result<BackendStatus, String> {
    let config = state.config.lock().await.clone();
    config.validate()?;
    if !can_launch_local() && config.launch_mode == LaunchMode::Local {
        return Err("this platform cannot spawn dsh web; switch to connect mode".into());
    }
    match config.launch_mode {
        LaunchMode::Connect => connect_existing(app, state, config).await,
        LaunchMode::Local => spawn_local(app, state, config).await,
    }
}

#[tauri::command]
async fn start_dsh(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<BackendStatus, String> {
    start_inner(&app, &state).await
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
    let _ = show_dsh_ui(app, &url);
    Ok(state.snapshot().await)
}

async fn spawn_local(
    app: &AppHandle,
    state: &AppState,
    config: DshConfig,
) -> Result<BackendStatus, String> {
    hide_dsh_ui(app);
    stop_inner(state).await?;
    let data = app_data_dir(app)?;
    let hints = runtime_hints(app);
    if needs_managed_install(&data, &hints) {
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
    if let Err(error) = install_starter_plugins(app, state, &data, &hints).await {
        let _ = app.emit("dsh-spawn-log", format!("starter plugins: {error}"));
    }
    set_state(app, state, StatusState::Starting, Some("Spawning dsh web".into())).await;
    let plan = resolve_launch(&data, &hints, &config)?;
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

    // The listen socket is up ~1s before `__DSH_BOOT__` includes connection/typert.
    // Navigating on TCP (or the ready line, which is close) loads a partial graph
    // and the WebView sticks on "N entries did not activate".
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut waiting_for_graph = false;
    let url = loop {
        if let Some(process) = state.process.lock().await.as_mut() {
            if let Ok(Some(status)) = process.try_wait() {
                scan_out.abort();
                scan_err.abort();
                let tail = tail_logs(&logs).await;
                let message = format!("dsh web exited ({status}) before it was ready.{tail}");
                hide_dsh_ui(app);
                set_state(app, state, StatusState::Error, Some(message.clone())).await;
                return Err(message);
            }
        }
        let candidate = found.lock().await.clone().or_else(|| {
            if config.port == 0 {
                None
            } else {
                config.web_url().ok()
            }
        });
        if let Some(candidate) = candidate {
            if let Some(html) = fetch_index(&candidate).await {
                if boot_manifest_ready(&html) {
                    break candidate;
                }
                if !waiting_for_graph {
                    waiting_for_graph = true;
                    let _ = app.emit(
                        "dsh-spawn-log",
                        "waiting for web boot graph (connection + typert)".to_string(),
                    );
                }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            scan_out.abort();
            scan_err.abort();
            stop_inner(state).await?;
            let tail = tail_logs(&logs).await;
            let message = format!("dsh web did not become ready within 90s.{tail}");
            hide_dsh_ui(app);
            set_state(app, state, StatusState::Error, Some(message.clone())).await;
            return Err(message);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    *state.url.lock().await = Some(url.clone());
    set_state(app, state, StatusState::Ready, Some("dsh web UI is ready".into())).await;
    let _ = show_dsh_ui(app, &url);
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
        if boot_ui_ready(url).await {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no ready dsh web UI at {url}"));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// GET `/` and require `__DSH_BOOT__` to include connection + typert.
/// The listen socket is up before client-modules finishes that scan.
async fn boot_ui_ready(url: &str) -> bool {
    fetch_index(url)
        .await
        .is_some_and(|html| boot_manifest_ready(&html))
}

async fn fetch_index(url: &str) -> Option<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_string();
    let port = parsed.port_or_known_default()?;
    let mut stream = tokio::time::timeout(
        Duration::from_millis(400),
        tokio::net::TcpStream::connect((host.as_str(), port)),
    )
    .await
    .ok()?
    .ok()?;
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\nAccept: text/html\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.ok()?;
    let mut buf = Vec::new();
    tokio::time::timeout(Duration::from_secs(2), stream.read_to_end(&mut buf))
        .await
        .ok()?
        .ok()?;
    String::from_utf8(buf).ok()
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
    hide_dsh_ui(&app);
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
    show_dsh_ui(&app, &url)
}

#[tauri::command]
async fn open_settings(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    hide_dsh_ui(&app);
    let _ = app.emit("dsh-open-settings", ());
    reveal_main(&app);
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let url = state.shell_url.lock().await.clone();
        if url.is_empty() {
            return Err("shell URL is not available".into());
        }
        navigate(&app, &settings_page_url(&url)?)?;
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = state;
    }
    Ok(())
}

#[tauri::command]
fn report_theme(app: AppHandle, bg: String, fg: String) {
    let _ = app.emit("dsh-theme", ThemePayload { bg, fg });
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
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let reload = MenuItemBuilder::with_id("open-web", "Open Web UI").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&settings)
        .item(&reload)
        .separator()
        .item(&quit)
        .build()?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("missing default window icon for tray")?;
    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("DeepSeek Harness")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                reveal_main(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => reveal_main(app),
            "settings" => {
                hide_dsh_ui(app);
                let _ = app.emit("dsh-open-settings", ());
                reveal_main(app);
            }
            "open-web" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let state = state.inner().clone();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(url) = state.url.lock().await.clone() {
                            let _ = show_dsh_ui(&app, &url);
                        }
                        reveal_main(&app);
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
            let state = Arc::new(AppState::new(config.clone()));
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
                inject_no_context_menu(&window);
            }
            app.manage(state.clone());
            if let Err(error) = build_tray(app.handle()) {
                eprintln!("dsh-desktop: tray unavailable: {error}");
            }
            if config.auto_start {
                if let Ok(mut slot) = state.state.try_lock() {
                    *slot = StatusState::Starting;
                }
                if let Ok(mut slot) = state.message.try_lock() {
                    *slot = Some("Starting dsh web".into());
                }
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = start_inner(&handle, &state).await {
                        set_state(&handle, &state, StatusState::Error, Some(error)).await;
                    }
                });
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
            report_theme,
            check_dsh_updates,
            update_dsh,
        ])
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    #[cfg(not(any(target_os = "android", target_os = "ios")))]
                    {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                tauri::WindowEvent::Resized(_) => {
                    #[cfg(not(any(target_os = "android", target_os = "ios")))]
                    {
                        if let Some(webview) = window.app_handle().get_webview(DSH_WEBVIEW) {
                            if let (Ok(size), Ok(scale)) =
                                (window.inner_size(), window.scale_factor())
                            {
                                let _ = layout_dsh_webview(size, scale, &webview);
                            }
                        }
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    if let Some(state) = window.app_handle().try_state::<Arc<AppState>>() {
                        tauri::async_runtime::block_on(async {
                            let _ = stop_inner(&state).await;
                        });
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running dsh-desktop");
}
