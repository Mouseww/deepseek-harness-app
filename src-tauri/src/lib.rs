use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_store::StoreExt;

/// DSH 后端进程管理器
#[derive(Default)]
pub struct DshBackend {
    process: Arc<Mutex<Option<Child>>>,
    port: Arc<Mutex<Option<u16>>>,
}

/// DSH 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DshConfig {
    pub host: String,
    pub port: u16,
    pub auto_start: bool,
}

impl Default for DshConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3080,
            auto_start: true,
        }
    }
}

/// 启动 DSH 后端（使用 npx @deepseek-ai/dsh）
#[tauri::command]
async fn start_dsh_backend(
    backend: State<'_, DshBackend>,
    config: DshConfig,
) -> Result<u16, String> {
    let mut process_lock = backend.process.lock().map_err(|e| e.to_string())?;

    // 如果已经在运行，返回当前端口
    if process_lock.is_some() {
        let port_lock = backend.port.lock().map_err(|e| e.to_string())?;
        if let Some(port) = *port_lock {
            return Ok(port);
        }
    }

    // 使用 npx 运行官方发布的 DSH 包
    let npx_cmd = if cfg!(windows) { "npx.cmd" } else { "npx" };

    let mut cmd = Command::new(npx_cmd);
    cmd.args(&[
        "@deepseek-ai/dsh",
        "web",
        "--host", &config.host,
        "--port", &config.port.to_string(),
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let child = cmd.spawn().map_err(|e| format!("Failed to start DSH: {}", e))?;

    *process_lock = Some(child);

    let mut port_lock = backend.port.lock().map_err(|e| e.to_string())?;
    *port_lock = Some(config.port);

    Ok(config.port)
}

/// 停止 DSH 后端
#[tauri::command]
async fn stop_dsh_backend(backend: State<'_, DshBackend>) -> Result<(), String> {
    let mut process_lock = backend.process.lock().map_err(|e| e.to_string())?;

    if let Some(mut child) = process_lock.take() {
        child.kill().map_err(|e| format!("Failed to kill DSH process: {}", e))?;
        child.wait().map_err(|e| format!("Failed to wait for DSH process: {}", e))?;
    }

    let mut port_lock = backend.port.lock().map_err(|e| e.to_string())?;
    *port_lock = None;

    Ok(())
}

/// 获取 DSH 后端状态
#[tauri::command]
async fn get_dsh_status(backend: State<'_, DshBackend>) -> Result<bool, String> {
    let process_lock = backend.process.lock().map_err(|e| e.to_string())?;
    Ok(process_lock.is_some())
}

/// 获取 DSH 端口
#[tauri::command]
async fn get_dsh_port(backend: State<'_, DshBackend>) -> Result<Option<u16>, String> {
    let port_lock = backend.port.lock().map_err(|e| e.to_string())?;
    Ok(*port_lock)
}

/// 获取 DSH 版本
#[tauri::command]
async fn get_dsh_version() -> Result<String, String> {
    let npx_cmd = if cfg!(windows) { "npx.cmd" } else { "npx" };

    let output = Command::new(npx_cmd)
        .args(&["@deepseek-ai/dsh", "--version"])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 检查 DSH 更新
#[tauri::command]
async fn check_dsh_updates() -> Result<String, String> {
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };

    let output = Command::new(npm_cmd)
        .args(&["view", "@deepseek-ai/dsh", "version"])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 更新 DSH
#[tauri::command]
async fn update_dsh(app: AppHandle) -> Result<(), String> {
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };

    let mut child = Command::new(npm_cmd)
        .args(&["install", "-g", "@deepseek-ai/dsh@latest"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    // 实时发送进度事件
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                app.emit("dsh-update-progress", line).ok();
            }
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;

    if status.success() {
        app.emit("dsh-update-complete", ()).ok();
        Ok(())
    } else {
        Err("DSH update failed".to_string())
    }
}

/// 获取配置
#[tauri::command]
async fn get_config(app: AppHandle) -> Result<DshConfig, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;

    let config = DshConfig {
        host: store
            .get("host")
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        port: store
            .get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(3080) as u16,
        auto_start: store
            .get("auto_start")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    };

    Ok(config)
}

/// 设置配置
#[tauri::command]
async fn set_config(app: AppHandle, config: DshConfig) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;

    store.set("host", serde_json::json!(config.host));
    store.set("port", serde_json::json!(config.port));
    store.set("auto_start", serde_json::json!(config.auto_start));

    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

/// 检查应用更新
#[tauri::command]
async fn check_app_updates(_app: AppHandle) -> Result<bool, String> {
    // TODO: 实现应用壳更新检查逻辑
    // 使用 tauri-plugin-updater
    Ok(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = try_run() {
        let message = format!("DeepSeek Harness failed to start:\n{error}");
        eprintln!("{message}");
        show_fatal_error("DeepSeek Harness App", &message);
        std::process::exit(1);
    }
}

fn try_run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .manage(DshBackend::default())
        .invoke_handler(tauri::generate_handler![
            start_dsh_backend,
            stop_dsh_backend,
            get_dsh_status,
            get_dsh_port,
            get_dsh_version,
            check_dsh_updates,
            update_dsh,
            get_config,
            set_config,
            check_app_updates,
        ])
        .setup(|_app| {
            // TODO: 实现系统托盘
            // TODO: 自动启动 DSH 后端（如果配置了 auto_start）
            Ok(())
        })
        .run(tauri::generate_context!())
}

fn show_fatal_error(title: &str, message: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        fn wide(value: &str) -> Vec<u16> {
            std::ffi::OsStr::new(value)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }

        const MB_ICONERROR: u32 = 0x00000010;
        extern "system" {
            fn MessageBoxW(
                hwnd: *mut core::ffi::c_void,
                text: *const u16,
                caption: *const u16,
                ty: u32,
            ) -> i32;
        }

        let text = wide(message);
        let caption = wide(title);
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                caption.as_ptr(),
                MB_ICONERROR,
            );
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (title, message);
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    /// Mirrors `tauri-plugin-shell` 2.x `Config` (`deny_unknown_fields`).
    /// The 1.0.0 MSI exited immediately because `plugins.shell.scope` is a
    /// Tauri 1 field that this struct rejects at plugin init.
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct ShellPluginConfig {
        #[serde(default)]
        #[allow(dead_code)]
        open: serde_json::Value,
    }

    #[test]
    fn tauri_shell_plugin_config_is_v2_compatible() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let shell = conf
            .pointer("/plugins/shell")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        serde_json::from_value::<ShellPluginConfig>(shell).expect(
            "plugins.shell must match Tauri 2 (only `open`; no Tauri 1 `scope`)",
        );
    }
}
