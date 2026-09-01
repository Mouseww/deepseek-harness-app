//! Resolve, spawn, stop, and update a managed dsh web process.
//!
//! The harness is the official npm package `@deepseek-ai/dsh`
//! (https://github.com/deepseek-ai/deepseek-harness). This crate never vendors that tree.

/// Published package this shell launches. Source: https://github.com/deepseek-ai/deepseek-harness
pub const DSH_PACKAGE: &str = "@deepseek-ai/dsh";
/// Pinned first-install spec. Matches `upstream.json` and package.json peerDependencies.
pub const DSH_PACKAGE_SPEC: &str = "@deepseek-ai/dsh@0.1.0-rc.7";

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::config::{web_launch_args, DshConfig};

/// A spawned local backend, if any.
pub struct DshProcess {
    child: Child,
}

impl DshProcess {
    /// Non-blocking poll. `Some` means the child has exited.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child.try_wait().map_err(|error| error.to_string())
    }

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

/// Resolve an official deepseek-harness checkout only when DSH_CHECKOUT is set.
/// Nearby clones are not used automatically: `pnpm dsh web` from source can sit in a
/// first compile for minutes and never print a ready line in time.
pub fn find_checkout(_start: &Path) -> Option<PathBuf> {
    let explicit = std::env::var("DSH_CHECKOUT").ok()?;
    let path = PathBuf::from(explicit);
    is_dsh_checkout(&path).then_some(path)
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
    pub env: Vec<(String, String)>,
}

fn node_bin_name() -> &'static str {
    if cfg!(windows) {
        "node.exe"
    } else {
        "node"
    }
}

fn dsh_bin_under(prefix: &Path) -> PathBuf {
    prefix
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js")
}

/// Bundled Node + `@deepseek-ai/dsh` shipped inside the installer.
pub fn bundled_runtime(root: &Path) -> Option<(PathBuf, PathBuf)> {
    let node = root.join("runtime").join("node").join(node_bin_name());
    let bin = dsh_bin_under(&root.join("runtime").join("dsh"));
    if node.is_file() && bin.is_file() {
        Some((node, bin))
    } else {
        None
    }
}

/// Search resource dir, then src-tauri/, then the given roots.
pub fn find_bundled_runtime(hints: &[PathBuf]) -> Option<(PathBuf, PathBuf)> {
    for hint in hints {
        if let Some(found) = bundled_runtime(hint) {
            return Some(found);
        }
        if let Some(found) = bundled_runtime(&hint.join("src-tauri")) {
            return Some(found);
        }
    }
    None
}

fn home_env(app_data: &Path) -> Vec<(String, String)> {
    vec![("DSH_HOME".into(), app_data.join("dsh-home").to_string_lossy().into_owned())]
}

/// `runtime/dsh` for `.../node_modules/@deepseek-ai/dsh/lib/bin.js`.
fn npm_prefix_from_bin(bin: &Path) -> Option<PathBuf> {
    Some(
        bin.parent()?
            .parent()?
            .parent()?
            .parent()?
            .parent()?
            .to_path_buf(),
    )
}

fn find_resolve_hook(hints: &[PathBuf]) -> Option<PathBuf> {
    for hint in hints {
        for candidate in [
            hint.join("hooks").join("resolve-register.mjs"),
            hint.join("src-tauri").join("hooks").join("resolve-register.mjs"),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Best-effort ancestor so Node's parent-walk from `$DSH_HOME/profiles/web`
/// can see the installation's `node_modules` even if profile healing lags.
fn ensure_home_node_modules(home: &Path, prefix: &Path) {
    let target = prefix.join("node_modules");
    if !target.is_dir() {
        return;
    }
    let _ = std::fs::create_dir_all(home);
    let link = home.join("node_modules");
    if link.exists() || link.symlink_metadata().is_ok() {
        return;
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(&target, &link).is_ok() {
            return;
        }
        let _ = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &target.to_string_lossy(),
            ])
            .creation_flags(0x0800_0000)
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::os::unix::fs::symlink(&target, &link);
    }
}

fn packaged_env(app_data: &Path, prefix: &Path) -> Vec<(String, String)> {
    let home = app_data.join("dsh-home");
    ensure_home_node_modules(&home, prefix);
    let mut env = home_env(app_data);
    env.push((
        "DSH_DESKTOP_PREFIX".into(),
        prefix.to_string_lossy().into_owned(),
    ));
    env.push((
        "NODE_PATH".into(),
        prefix.join("node_modules").to_string_lossy().into_owned(),
    ));
    env
}

/// Tauri's resource dir is often `\\?\D:\...`. Node rejects `file:////?/D:/`.
fn normalize_os_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    let _ = text;
    path.to_path_buf()
}

fn path_to_file_url(path: &Path) -> String {
    let path = normalize_os_path(path);
    match url::Url::from_file_path(&path) {
        Ok(url) => url.into(),
        Err(()) => {
            let raw = path.to_string_lossy().replace('\\', "/");
            if raw.starts_with('/') {
                format!("file://{raw}")
            } else {
                format!("file:///{raw}")
            }
        }
    }
}

fn node_cli_args(bin: &Path, hook: Option<&Path>, cli: &[String]) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(hook) = hook {
        args.push("--import".into());
        args.push(path_to_file_url(hook));
    }
    args.push(bin.to_string_lossy().into_owned());
    args.extend(cli.iter().cloned());
    args
}

/// First-launch plugins installed into the official `web` profile.
pub const STARTER_PLUGINS: &[(&str, &str)] = &[
    ("dsh-web-ui", "github:zhu1090093659/dsh-web-ui"),
    ("Transparent UI", "github:WYH66666666/DSH-Transparent-UI-Plugin"),
    ("better-sidebar", "github:omdsh-dev/DSH-better-sidebar"),
    ("dsh-visualize", "github:Nagi-ovo/dsh-visualize"),
];

/// npm package names that `dsh plugin add` writes into the profile.
/// GitHub specs install as these names, then the loader imports them as
/// bundles from the profile `package.json`.
pub const STARTER_PLUGIN_PACKAGES: &[&str] = &[
    "dsh-web-ui",
    "@deepseek-ai/dsh-client-ui-aqua",
    "dsh-better-sidebar",
    "@dsh-external/dsh-visualize",
];

/// `dsh plugin --profile web add <spec>`
pub fn plugin_add_args(spec: &str) -> Vec<String> {
    vec![
        "plugin".into(),
        "--profile".into(),
        "web".into(),
        "add".into(),
        spec.to_string(),
    ]
}

/// True when an already-present plugin should not fail first launch.
pub fn plugin_already_present(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("already") || error.contains("exists") || error.contains("duplicate")
}

fn profile_package_path(app_data: &Path) -> PathBuf {
    app_data
        .join("dsh-home")
        .join("profiles")
        .join("web")
        .join("package.json")
}

/// True when the named npm package exists under the official `web` profile.
pub fn plugin_installed_on_disk(app_data: &Path, package: &str) -> bool {
    let root = app_data
        .join("dsh-home")
        .join("profiles")
        .join("web")
        .join("node_modules");
    if let Some(rest) = package.strip_prefix('@') {
        if let Some((scope, name)) = rest.split_once('/') {
            return root
                .join(format!("@{scope}"))
                .join(name)
                .join("package.json")
                .is_file();
        }
    }
    root.join(package).join("package.json").is_file()
}

/// Drop profile bundles whose packages are not actually present on disk.
/// Official dsh treats a missing bundle as a fatal plugin-tree failure.
pub fn prune_missing_profile_bundles(app_data: &Path) -> Result<Vec<String>, String> {
    let path = profile_package_path(app_data);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let Some(bundles) = value
        .pointer_mut("/dsh/profile/bundles")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Ok(Vec::new());
    };
    let mut removed = Vec::new();
    bundles.retain(|entry| {
        let Some(name) = entry.as_str() else {
            return true;
        };
        if name.starts_with("@deepseek-ai/") {
            return true;
        }
        if STARTER_PLUGIN_PACKAGES.contains(&name) && !plugin_installed_on_disk(app_data, name) {
            removed.push(name.to_string());
            return false;
        }
        true
    });
    if removed.is_empty() {
        return Ok(removed);
    }
    if let Some(map) = value.get_mut("dependencies").and_then(|deps| deps.as_object_mut()) {
        for name in &removed {
            map.remove(name);
        }
    }
    let pretty = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
    std::fs::write(&path, pretty + "\n").map_err(|error| error.to_string())?;
    Ok(removed)
}

/// True when there is no bundled runtime and the app-data prefix still needs npm.
pub fn needs_managed_install(app_data: &Path, hints: &[PathBuf]) -> bool {
    std::env::var("DSH_DESKTOP_BIN").is_err()
        && find_checkout(app_data).is_none()
        && find_bundled_runtime(hints).is_none()
        && managed_bin(&runtime_prefix(app_data)).is_none()
}

/// Resolve node/pnpm/dsh for an arbitrary CLI after the launcher token.
pub fn resolve_cli(
    app_data: &Path,
    hints: &[PathBuf],
    cli: &[String],
) -> Result<LaunchPlan, String> {
    let home = app_data.join("dsh-home");
    if let Ok(explicit) = std::env::var("DSH_DESKTOP_BIN") {
        return Ok(LaunchPlan {
            program: PathBuf::from(explicit),
            args: cli.to_vec(),
            cwd: Some(home),
            env: home_env(app_data),
        });
    }
    if let Some((node, bin)) = find_bundled_runtime(hints) {
        let prefix = npm_prefix_from_bin(&bin).unwrap_or_else(|| bin.parent().unwrap_or(&bin).to_path_buf());
        return Ok(LaunchPlan {
            program: node,
            args: node_cli_args(&bin, find_resolve_hook(hints).as_deref(), cli),
            cwd: Some(prefix.clone()),
            env: packaged_env(app_data, &prefix),
        });
    }
    if let Some(root) = find_checkout(app_data) {
        let pnpm = which("pnpm.cmd")
            .or_else(|| which("pnpm"))
            .ok_or_else(|| "pnpm not found on PATH for DSH_CHECKOUT".to_string())?;
        let mut args = vec!["dsh".into()];
        args.extend(cli.iter().cloned());
        return Ok(LaunchPlan {
            program: pnpm,
            args,
            cwd: Some(root),
            env: home_env(app_data),
        });
    }
    let node = find_node().ok_or_else(|| {
        "This build has no bundled Node, and Node.js 22.19+ is not on PATH. Use a packaged installer or install Node."
            .to_string()
    })?;
    let prefix = runtime_prefix(app_data);
    let bin = managed_bin(&prefix).ok_or_else(|| {
        format!("managed {DSH_PACKAGE} is not installed yet")
    })?;
    Ok(LaunchPlan {
        program: node,
        args: node_cli_args(&bin, find_resolve_hook(hints).as_deref(), cli),
        cwd: Some(prefix.clone()),
        env: packaged_env(app_data, &prefix),
    })
}

/// Choose bundled runtime, DSH_DESKTOP_BIN, an explicit checkout, or managed `node` + bin.js.
pub fn resolve_launch(
    app_data: &Path,
    hints: &[PathBuf],
    config: &DshConfig,
) -> Result<LaunchPlan, String> {
    let mut cli = vec!["web".into()];
    cli.extend(web_launch_args(&config.host, config.port));
    resolve_cli(app_data, hints, &cli)
}

/// Human-readable command line for the settings log.
pub fn launch_command_line(plan: &LaunchPlan) -> String {
    let mut parts = vec![plan.program.display().to_string()];
    parts.extend(plan.args.iter().cloned());
    parts.join(" ")
}

fn apply_stdio(cmd: &mut Command) {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}

fn command_for(program: &Path, args: &[String]) -> Command {
    #[cfg(windows)]
    {
        let ext = program
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "cmd" || ext == "bat" {
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/D").arg("/C").arg(program).args(args);
            return cmd;
        }
    }
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd
}

/// Spawn the resolved command with piped stdio.
pub fn spawn_plan(plan: &LaunchPlan) -> Result<Child, String> {
    let mut cmd = command_for(&plan.program, &plan.args);
    apply_stdio(&mut cmd);
    if let Some(cwd) = &plan.cwd {
        let _ = std::fs::create_dir_all(cwd);
        cmd.current_dir(cwd);
    }
    for (key, value) in &plan.env {
        cmd.env(key, value);
    }
    cmd.spawn()
        .map_err(|error| format!("spawn {}: {error}", plan.program.display()))
}

/// Run a one-shot CLI (plugin add, etc.) to completion, streaming lines.
pub async fn run_plan<F>(plan: &LaunchPlan, on_line: F) -> Result<(), String>
where
    F: Fn(String) + Send + Sync + 'static,
{
    let mut child = spawn_plan(plan)?;
    let on_line = std::sync::Arc::new(std::sync::Mutex::new(on_line));
    let mut tasks = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let callback = on_line.clone();
        tasks.push(tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(emit) = callback.lock() {
                    emit(line);
                }
            }
        }));
    }
    if let Some(stderr) = child.stderr.take() {
        let callback = on_line.clone();
        tasks.push(tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(emit) = callback.lock() {
                    emit(line);
                }
            }
        }));
    }
    let status = child.wait().await.map_err(|error| error.to_string())?;
    for task in tasks {
        let _ = task.await;
    }
    if !status.success() {
        return Err(format!("command failed with {status}"));
    }
    Ok(())
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
    let args = ["view".into(), DSH_PACKAGE.into(), "version".into()];
    let output = command_for(&npm, &args)
        .output()
        .await
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Install or upgrade the managed prefix with `npm install <spec>`.
pub async fn install_managed<F>(prefix: &Path, spec: &str, mut on_line: F) -> Result<(), String>
where
    F: FnMut(String),
{
    let npm = find_npm().ok_or_else(|| "npm not found on PATH".to_string())?;
    std::fs::create_dir_all(prefix).map_err(|error| error.to_string())?;
    let args = [
        "install".into(),
        spec.to_string(),
        "--omit=dev".into(),
        "--no-fund".into(),
        "--no-audit".into(),
        "--loglevel".into(),
        "info".into(),
    ];
    let mut child = command_for(&npm, &args);
    apply_stdio(&mut child);
    child.current_dir(prefix);
    let mut child = child.spawn().map_err(|error| error.to_string())?;
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
        assert_eq!(find_checkout(&tmp.join("apps").join("desktop")), None);
        assert!(bundled_runtime(&tmp).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn plugin_add_args_match_official_cli() {
        assert_eq!(
            plugin_add_args("github:zhu1090093659/dsh-web-ui"),
            vec![
                "plugin",
                "--profile",
                "web",
                "add",
                "github:zhu1090093659/dsh-web-ui"
            ]
        );
    }

    #[test]
    fn starter_plugins_are_the_four_web_profile_defaults() {
        let specs: Vec<&str> = STARTER_PLUGINS.iter().map(|(_, spec)| *spec).collect();
        assert_eq!(
            specs,
            vec![
                "github:zhu1090093659/dsh-web-ui",
                "github:WYH66666666/DSH-Transparent-UI-Plugin",
                "github:omdsh-dev/DSH-better-sidebar",
                "github:Nagi-ovo/dsh-visualize",
            ]
        );
    }

    #[test]
    fn duplicate_plugin_errors_are_benign() {
        assert!(plugin_already_present("Plugin already installed"));
        assert!(plugin_already_present("entry exists in profile"));
        assert!(!plugin_already_present("network timeout"));
    }

    #[test]
    fn prune_missing_profile_bundles_drops_absent_starter_plugins() {
        let tmp = std::env::temp_dir().join(format!(
            "dsh-desktop-prune-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let profile = tmp.join("dsh-home").join("profiles").join("web");
        std::fs::create_dir_all(profile.join("node_modules").join("dsh-web-ui")).unwrap();
        std::fs::write(
            profile.join("node_modules").join("dsh-web-ui").join("package.json"),
            "{\"name\":\"dsh-web-ui\"}\n",
        )
        .unwrap();
        std::fs::write(
            profile.join("package.json"),
            r#"{
  "name": "dsh-profile-web",
  "private": true,
  "dependencies": {
    "dsh-web-ui": "github:zhu1090093659/dsh-web-ui",
    "dsh-better-sidebar": "github:omdsh-dev/DSH-better-sidebar",
    "@dsh-external/dsh-visualize": "github:Nagi-ovo/dsh-visualize"
  },
  "dsh": {
    "profile": {
      "bundles": [
        "@deepseek-ai/dsh-base",
        "@deepseek-ai/dsh-web-app",
        "dsh-better-sidebar",
        "@dsh-external/dsh-visualize"
      ]
    }
  }
}
"#,
        )
        .unwrap();
        let removed = prune_missing_profile_bundles(&tmp).unwrap();
        assert_eq!(
            removed,
            vec![
                "dsh-better-sidebar".to_string(),
                "@dsh-external/dsh-visualize".to_string()
            ]
        );
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(profile.join("package.json")).unwrap())
                .unwrap();
        let bundles = value
            .pointer("/dsh/profile/bundles")
            .and_then(|v| v.as_array())
            .unwrap();
        let names: Vec<&str> = bundles.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(
            names,
            vec!["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"]
        );
        assert!(value
            .pointer("/dependencies/dsh-better-sidebar")
            .is_none());
        assert!(value
            .pointer("/dependencies/@dsh-external/dsh-visualize")
            .is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn plugin_installed_on_disk_checks_scoped_and_bare_packages() {
        let tmp = std::env::temp_dir().join(format!(
            "dsh-desktop-installed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = tmp
            .join("dsh-home")
            .join("profiles")
            .join("web")
            .join("node_modules");
        std::fs::create_dir_all(root.join("@dsh-external").join("dsh-visualize")).unwrap();
        std::fs::write(
            root.join("@dsh-external")
                .join("dsh-visualize")
                .join("package.json"),
            "{}\n",
        )
        .unwrap();
        assert!(plugin_installed_on_disk(&tmp, "@dsh-external/dsh-visualize"));
        assert!(!plugin_installed_on_disk(&tmp, "dsh-better-sidebar"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn starter_plugin_package_names_align_with_specs() {
        assert_eq!(STARTER_PLUGINS.len(), STARTER_PLUGIN_PACKAGES.len());
        assert_eq!(STARTER_PLUGIN_PACKAGES[2], "dsh-better-sidebar");
        assert_eq!(STARTER_PLUGIN_PACKAGES[3], "@dsh-external/dsh-visualize");
    }
}
