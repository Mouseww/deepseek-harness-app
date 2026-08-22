//! Check GitHub Releases for a newer DSH Desktop installer and run it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

/// Public repo that publishes desktop installers.
pub const UPDATE_REPO: &str = "Mouseww/deepseek-harness-app";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateStatus {
    pub state: AppUpdateState,
    pub current: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_name: Option<String>,
    pub bytes_downloaded: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppUpdateState {
    Idle,
    Checking,
    Available,
    Downloading,
    Installing,
    Error,
}

impl AppUpdateStatus {
    pub fn current_only() -> Self {
        Self {
            state: AppUpdateState::Idle,
            current: env!("CARGO_PKG_VERSION").into(),
            latest: None,
            notes_url: None,
            asset_name: None,
            bytes_downloaded: 0,
            bytes_total: None,
            message: None,
            available: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Parse `1.4.0` / `v1.4.0` into comparable triples.
pub fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches('v').trim_start_matches('V');
    let mut parts = trimmed.split('.');
    let major = parse_numeric_prefix(parts.next()?)?;
    let minor = parse_numeric_prefix(parts.next()?)?;
    let patch = parse_numeric_prefix(parts.next().unwrap_or("0"))?;
    Some((major, minor, patch))
}

fn parse_numeric_prefix(part: &str) -> Option<u64> {
    let digits: String = part.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

/// True when `latest` is a higher semver than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(left), Some(right)) => left > right,
        _ => false,
    }
}

/// Choose the installer GitHub asset for this OS/arch.
pub fn pick_asset<'a>(assets: &'a [GhAsset], os: &str, arch: &str) -> Option<&'a GhAsset> {
    let mut best: Option<(&'a GhAsset, i32)> = None;
    for asset in assets {
        let name = asset.name.to_ascii_lowercase();
        let score = asset_score(&name, os, arch);
        if score <= 0 {
            continue;
        }
        if best.map(|(_, prev)| score > prev).unwrap_or(true) {
            best = Some((asset, score));
        }
    }
    best.map(|(asset, _)| asset)
}

fn asset_score(name: &str, os: &str, arch: &str) -> i32 {
    if name.ends_with(".blockmap")
        || name.ends_with(".sig")
        || name.ends_with(".json")
        || name.ends_with(".zip")
        || name.ends_with(".tar.gz")
        || name.contains("debug")
    {
        return 0;
    }
    match os {
        "windows" => score_windows(name, arch),
        "macos" => score_macos(name, arch),
        "linux" => score_linux(name, arch),
        _ => 0,
    }
}

fn score_windows(name: &str, arch: &str) -> i32 {
    if !name.ends_with(".exe") && !name.ends_with(".msi") {
        return 0;
    }
    if arch == "x86_64" && is_arm_name(name) {
        return 0;
    }
    let mut score = 10;
    if name.contains("setup") {
        score += 20;
    }
    if name.ends_with(".exe") {
        score += 8;
    }
    if matches_arch(name, arch) {
        score += 15;
    }
    score
}

fn score_macos(name: &str, arch: &str) -> i32 {
    if !name.ends_with(".dmg") {
        return 0;
    }
    let mut score = 10;
    if name.contains("universal") {
        score += 12;
    }
    if matches_arch(name, arch) {
        score += 18;
    } else if arch == "x86_64" && is_arm_name(name) {
        return 0;
    }
    score
}

fn score_linux(name: &str, arch: &str) -> i32 {
    if !name.ends_with(".deb") {
        return 0;
    }
    let mut score = 10;
    if matches_arch(name, arch) {
        score += 15;
    }
    score
}

fn is_arm_name(name: &str) -> bool {
    name.contains("arm64") || name.contains("aarch64")
}

fn matches_arch(name: &str, arch: &str) -> bool {
    match arch {
        "x86_64" | "amd64" => {
            name.contains("x64") || name.contains("x86_64") || name.contains("amd64")
        }
        "aarch64" => is_arm_name(name),
        _ => false,
    }
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(format!("dsh-desktop/{}", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| error.to_string())
}

/// Query GitHub for the latest published installer.
pub async fn fetch_latest(os: &str, arch: &str) -> Result<AppUpdateStatus, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let url = format!("https://api.github.com/repos/{UPDATE_REPO}/releases/latest");
    let release: GhRelease = http_client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let latest = release.tag_name.trim().trim_start_matches('v').to_string();
    let available = is_newer(&latest, &current);
    let asset = if available {
        pick_asset(&release.assets, os, arch)
    } else {
        None
    };
    if available && asset.is_none() {
        return Ok(AppUpdateStatus {
            state: AppUpdateState::Error,
            current,
            latest: Some(latest),
            notes_url: Some(release.html_url),
            asset_name: None,
            bytes_downloaded: 0,
            bytes_total: None,
            message: Some(format!("No installer for {os}/{arch} in the latest release")),
            available: false,
        });
    }
    Ok(AppUpdateStatus {
        state: if available {
            AppUpdateState::Available
        } else {
            AppUpdateState::Idle
        },
        current,
        latest: Some(latest),
        notes_url: Some(release.html_url),
        asset_name: asset.map(|item| item.name.clone()),
        bytes_downloaded: 0,
        bytes_total: asset.map(|item| item.size),
        message: if available {
            Some("A newer desktop build is available.".into())
        } else {
            Some("This app is up to date.".into())
        },
        available,
    })
}

/// Download the matching installer into the temp dir.
pub async fn download_installer<F>(
    os: &str,
    arch: &str,
    mut on_progress: F,
) -> Result<(PathBuf, AppUpdateStatus), String>
where
    F: FnMut(u64, Option<u64>),
{
    let url = format!("https://api.github.com/repos/{UPDATE_REPO}/releases/latest");
    let release: GhRelease = http_client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let asset = pick_asset(&release.assets, os, arch).ok_or_else(|| {
        format!("no installer for {os}/{arch} in {}", release.tag_name)
    })?;
    let dest = std::env::temp_dir().join(&asset.name);
    let client = http_client()?;
    let mut response = client
        .get(&asset.browser_download_url)
        .timeout(Duration::from_secs(300))
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let total = response.content_length().or(Some(asset.size));
    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|error| error.to_string())?;
    let mut downloaded = 0_u64;
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        file.write_all(&chunk)
            .await
            .map_err(|error| error.to_string())?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }
    file.flush().await.map_err(|error| error.to_string())?;
    let status = AppUpdateStatus {
        state: AppUpdateState::Installing,
        current: env!("CARGO_PKG_VERSION").into(),
        latest: Some(release.tag_name.trim().trim_start_matches('v').into()),
        notes_url: Some(release.html_url),
        asset_name: Some(asset.name.clone()),
        bytes_downloaded: downloaded,
        bytes_total: total,
        message: Some("Installer downloaded; launching.".into()),
        available: true,
    };
    Ok((dest, status))
}

/// Open the downloaded installer with the OS handler.
pub fn launch_installer(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new(path)
            .spawn()
            .map_err(|error| format!("launch installer: {error}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|error| format!("open installer: {error}"))?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|error| format!("open installer: {error}"))?;
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        Err("in-app update is desktop-only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> GhAsset {
        GhAsset {
            name: name.into(),
            browser_download_url: format!("https://example.test/{name}"),
            size: 10,
        }
    }

    #[test]
    fn version_compare_treats_v_prefix() {
        assert_eq!(parse_version("v1.4.0"), Some((1, 4, 0)));
        assert!(is_newer("1.4.1", "1.4.0"));
        assert!(is_newer("v1.5.0", "1.4.9"));
        assert!(!is_newer("1.4.0", "1.4.0"));
        assert!(!is_newer("1.3.9", "1.4.0"));
    }

    #[test]
    fn windows_prefers_nsis_setup_exe() {
        let assets = [
            asset("DSH-Desktop_1.4.1_x64_en-US.msi"),
            asset("DSH-Desktop_1.4.1_x64-setup.exe"),
            asset("DSH-Desktop_1.4.1_x64-setup.exe.blockmap"),
        ];
        assert_eq!(
            pick_asset(&assets, "windows", "x86_64").map(|item| item.name.as_str()),
            Some("DSH-Desktop_1.4.1_x64-setup.exe")
        );
    }

    #[test]
    fn macos_skips_wrong_arch() {
        let assets = [
            asset("DSH-Desktop_1.4.1_aarch64.dmg"),
            asset("DSH-Desktop_1.4.1_x64.dmg"),
        ];
        assert_eq!(
            pick_asset(&assets, "macos", "aarch64").map(|item| item.name.as_str()),
            Some("DSH-Desktop_1.4.1_aarch64.dmg")
        );
    }

    #[test]
    fn linux_picks_deb() {
        let assets = [asset("dsh-desktop_1.4.1_amd64.deb"), asset("notes.json")];
        assert_eq!(
            pick_asset(&assets, "linux", "x86_64").map(|item| item.name.as_str()),
            Some("dsh-desktop_1.4.1_amd64.deb")
        );
    }
}
