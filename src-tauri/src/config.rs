//! Persisted host/port and launch-mode settings.

use serde::{Deserialize, Serialize};

/// How the shell reaches DSH web.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchMode {
    /// Spawn a local dsh web process.
    Local,
    /// Navigate to an already-running server.
    Connect,
}

/// Settings stored in the Tauri store as config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DshConfig {
    /// Bind or connect host.
    pub host: String,
    /// Listen or connect port. 0 asks the OS for a free port on local launch.
    pub port: u16,
    /// Start or connect automatically when the window opens.
    pub auto_start: bool,
    /// Spawn locally, or only navigate.
    pub launch_mode: LaunchMode,
}

impl Default for DshConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3080,
            auto_start: true,
            launch_mode: LaunchMode::Local,
        }
    }
}

impl DshConfig {
    /// Reject values the shell or dsh web cannot use.
    pub fn validate(&self) -> Result<(), String> {
        let host = self.host.trim();
        if host.is_empty() {
            return Err("host must not be empty".into());
        }
        if host.contains(char::is_whitespace) || host.contains('/') || host.contains(':') {
            return Err(
                "host must be a hostname or IPv4 literal, without a scheme or port".into(),
            );
        }
        if self.launch_mode == LaunchMode::Local && host == "0.0.0.0" {
            return Err(
                "host 0.0.0.0 is not supported: dsh web refuses all-interfaces binds; use 127.0.0.1"
                    .into(),
            );
        }
        if self.launch_mode == LaunchMode::Connect && self.port == 0 {
            return Err("connect mode needs an explicit nonzero port".into());
        }
        Ok(())
    }

    /// http://host:port/ for a concrete port.
    pub fn web_url(&self) -> Result<String, String> {
        if self.port == 0 {
            return Err("cannot build a URL until dsh web reports its assigned port".into());
        }
        Ok(format!("http://{}:{}/", self.host.trim(), self.port))
    }
}

/// Extract the listen URL from one dsh web stdout line.
pub fn parse_ready_line(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("dsh web: ")?;
    let url = rest.split_whitespace().next()?;
    if url.starts_with("http://") || url.starts_with("https://") {
        Some(url.to_string())
    } else {
        None
    }
}

/// --host / --port suffix after the launcher tokens.
pub fn web_launch_args(host: &str, port: u16) -> [String; 4] {
    [
        "--host".into(),
        host.to_string(),
        "--port".into(),
        port.to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(DshConfig::default().validate().is_ok());
        assert_eq!(
            DshConfig::default().web_url().unwrap(),
            "http://127.0.0.1:3080/"
        );
    }

    #[test]
    fn connect_rejects_port_zero() {
        let cfg = DshConfig {
            launch_mode: LaunchMode::Connect,
            port: 0,
            ..DshConfig::default()
        };
        assert!(cfg.validate().unwrap_err().contains("nonzero"));
    }

    #[test]
    fn local_rejects_all_interfaces() {
        let cfg = DshConfig {
            host: "0.0.0.0".into(),
            ..DshConfig::default()
        };
        assert!(cfg.validate().unwrap_err().contains("0.0.0.0"));
    }

    #[test]
    fn ready_line_reads_loopback_url() {
        assert_eq!(
            parse_ready_line("dsh web: http://127.0.0.1:4567 (LAN: http://192.168.1.5:4567)")
                .as_deref(),
            Some("http://127.0.0.1:4567")
        );
        assert!(parse_ready_line("waiting").is_none());
    }
}
