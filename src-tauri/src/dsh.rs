//! Resolve, spawn, stop, and update a managed dsh web process.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::config::{web_launch_args, DshConfig};

/// A spawned local backend, if any.
pub struct DshProcess {
    child: Child,
}

impl DshProcess {
    /// Kill the process tree. Windows uses taskkill /T; elsewhere SIGKILL on the child.
    pub async fn stop(&mut self) -> Result<(), String> {
        let Some(pid) = self.child.id() else {
            return Ok(());
        };
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status()
                .await;
        }
        #[cfg(not(windows))]
        {
            let _ = pid;
            let _ = self.child.start_kill();
        }
        let _ = tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await;
        Ok(())
    }
}

/// Whether this OS can spawn Node/dsh at all.
pub fn can_launch_local() -> bool {
    !cfg!(any(target_os = "android", target_os = "ios"))
}

/// Locate a Node executable, or None when none is on PATH / DSH_DESKTOP_NODE.
pub fn find_node() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("DSH_DESKTOP_NODE") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Some(path);
        }
    }
    which("node").or_else(|| which("node.exe"))
}

/// Locate npm / npm.cmd.
pub fn find_npm() -> Option<PathBuf> {
    which("npm.cmd").or_else(|| which("npm"))
}

fn which(name: &str) -> Option<PathBuf> {
    let Ok(path) = std::env::var("PATH") else {
        return None;
    };
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn is_dsh_checkout(dir: &Path) -> bool {
    dir.join("pnpm-workspace.yaml").is_file() && dir.join("apps").join("cli").is_dir()
}

/// Resolve an official deepseek-harness checkout. This shell never vendors that tree.
/// Order: DSH_CHECKOUT, a sibling directory named deepseek-harness, then walk up from start.
pub fn find_checkout(start: &Path) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("DSH_CHECKOUT") {
        let path = PathBuf::from(explicit);
        if is_dsh_checkout(&path) {
            return Some(path);
        }
    }
    if let Some(parent) = start.parent() {
        let sibling = parent.join("deepseek-harness");
        if is_dsh_checkout(&sibling) {
            return Some(sibling);
        }
    }
    let mut current = Some(start);
    while let Some(dir) = current {
        if is_dsh_checkout(dir) {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

/// App-data directory that holds the managed npm prefix.
pub fn runtime_prefix(app_data: &Path) -> PathBuf {
    app_data.join("dsh-runtime")
}

/// node .../lib/bin.js for a managed install, when present.
pub fn managed_bin(prefix: &Path) -> Option<PathBuf> {
    let bin = prefix
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js");
    bin.is_file().then_some(bin)
}

/// Read version from the managed package manifest.
pub fn managed_version(prefix: &Path) -> Option<String> {
    let manifest = prefix
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("package.json");
    let text = std::fs::read_to_string(manifest).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("version")?.as_str().map(str::to_string)
}

/// One resolved local launch command.
pub struct LaunchPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

/// Choose checkout pnpm dsh, a managed bin, PATH dsh, or npx.
pub fn resolve_launch(app_data: &Path, cwd: &Path, config: &DshConfig) -> Result<LaunchPlan, String> {
    let flags = web_launch_args(&config.host, config.port);
    if let Ok(explicit) = std::env::var("DSH_DESKTOP_BIN") {
        let mut args = vec!["web".into()];
        args.extend(flags);
        return Ok(LaunchPlan {
            program: PathBuf::from(explicit),
            args,
            cwd: None,
        });
    }
    if let Some(root) = find_checkout(cwd).or_else(|| find_checkout(app_data)) {
        let pnpm = which("pnpm.cmd")
            .or_else(|| which("pnpm"))
            .ok_or_else(|| "pnpm not found on PATH for this checkout".to_string())?;
        let mut args = vec!["dsh".into(), "web".into()];
        args.extend(flags);
        return Ok(LaunchPlan {
            program: pnpm,
            args,
            cwd: Some(root),
        });
    }
    if let (Some(node), Some(bin)) = (find_node(), managed_bin(&runtime_prefix(app_data))) {
        let mut args = vec![bin.to_string_lossy().into_owned(), "web".into()];
        args.extend(flags);
        return Ok(LaunchPlan {
            program: node,
            args,
            cwd: None,
        });
    }
    if let Some(dsh) = which("dsh.cmd").or_else(|| which("dsh")) {
        let mut args = vec!["web".into()];
        args.extend(flags);
        return Ok(LaunchPlan {
            program: dsh,
            args,
            cwd: None,
        });
    }
    let npx = which("npx.cmd").or_else(|| which("npx")).ok_or_else(|| {
        "no dsh, managed install, or npx found; install Node.js 22.19+ or set DSH_DESKTOP_BIN"
            .to_string()
    })?;
    let mut args = vec![
        "--yes".into(),
        "@deepseek-ai/dsh".into(),
        "web".into(),
    ];
    args.extend(flags);
    Ok(LaunchPlan {
        program: npx,
        args,
        cwd: None,
    })
}

/// Spawn the resolved command with piped stdio.
pub fn spawn_plan(plan: &LaunchPlan) -> Result<Child, String> {
    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(cwd) = &plan.cwd {
        cmd.current_dir(cwd);
    }
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.spawn()
        .map_err(|error| format!("spawn {}: {error}", plan.program.display()))
}

/// Wrap a spawned child so callers can stop it later.
pub fn wrap_child(child: Child) -> DshProcess {
    DshProcess { child }
}

/// Take stdout/stderr from a live child for ready-line scanning.
pub fn take_pipes(
    process: &mut DshProcess,
) -> Result<(tokio::process::ChildStdout, tokio::process::ChildStderr), String> {
    let stdout = process
        .child
        .stdout
        .take()
        .ok_or_else(|| "dsh web stdout already taken".to_string())?;
    let stderr = process
        .child
        .stderr
        .take()
        .ok_or_else(|| "dsh web stderr already taken".to_string())?;
    Ok((stdout, stderr))
}

/// Read lines from a pipe and call on_line until it returns true or the pipe ends.
pub async fn scan_lines<R, F>(reader: R, mut on_line: F) -> Result<(), String>
where
    R: tokio::io::AsyncRead + Unpin,
    F: FnMut(&str) -> bool,
{
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await.map_err(|error| error.to_string())? {
        if on_line(&line) {
            break;
        }
    }
    Ok(())
}

/// npm view @deepseek-ai/dsh version
pub async fn registry_version() -> Result<String, String> {
    let npm = find_npm().ok_or_else(|| "npm not found on PATH".to_string())?;
    let output = Command::new(npm)
        .args(["view", "@deepseek-ai/dsh", "version"])
        .output()
        .await
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Install or upgrade the managed prefix with npm install @deepseek-ai/dsh@latest.
pub async fn install_managed<F>(prefix: &Path, mut on_line: F) -> Result<(), String>
where
    F: FnMut(String),
{
    let npm = find_npm().ok_or_else(|| "npm not found on PATH".to_string())?;
    std::fs::create_dir_all(prefix).map_err(|error| error.to_string())?;
    let mut child = Command::new(npm)
        .current_dir(prefix)
        .args([
            "install",
            "@deepseek-ai/dsh@latest",
            "--omit=dev",
            "--no-fund",
            "--no-audit",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            on_line(line);
        }
    }
    if let Some(stderr) = child.stderr.take() {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            on_line(line);
        }
    }
    let status = child.wait().await.map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!("npm install failed with {status}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_probe_needs_workspace_and_cli() {
        let tmp = std::env::temp_dir().join(format!("dsh-desktop-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("apps").join("cli")).unwrap();
        std::fs::write(tmp.join("pnpm-workspace.yaml"), "packages: []\n").unwrap();
        assert_eq!(find_checkout(&tmp.join("apps").join("desktop")), Some(tmp.clone()));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
