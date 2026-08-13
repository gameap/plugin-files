//! Node installation state and per-node FTP/SFTP configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupStatus {
    NotInstalled,
    Installing,
    Installed,
    Failed,
}

impl SetupStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotInstalled => "not_installed",
            Self::Installing => "installing",
            Self::Installed => "installed",
            Self::Failed => "failed",
        }
    }
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

/// Stored under NODE-scoped key `ftp:setup_status`. omitempty semantics match
/// the Go struct exactly; `last_check` is always emitted (Go had no omitempty
/// on it), while the HTTP status DTO omits it — keep the two apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSetupStatus {
    pub status: SetupStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub task_id: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error_message: String,
    #[serde(default)]
    pub last_check: i64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub started_at: i64,
    /// The get-tool task chained before the install task. New in the Rust
    /// port (task events are matched by id, not by the never-matching task
    /// type prefix the Go version filtered on). Absent in Go-written docs.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub download_task_id: u64,
}

impl NodeSetupStatus {
    pub fn new(status: SetupStatus) -> Self {
        Self {
            status,
            version: String::new(),
            task_id: 0,
            error_message: String::new(),
            last_check: 0,
            started_at: 0,
            download_task_id: 0,
        }
    }
}

/// Stored under NODE-scoped key `ftp:node_config`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ftp: Option<FtpConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sftp: Option<SftpConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FtpConfig {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub port: i64,
    #[serde(default)]
    pub passive_port_min: i64,
    #[serde(default)]
    pub passive_port_max: i64,
    #[serde(default)]
    pub public_host: String,
    #[serde(default)]
    pub tls_enabled: bool,
    #[serde(default)]
    pub tls_implicit_port: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SftpConfig {
    #[serde(default)]
    pub port: i64,
}

pub fn default_node_config() -> NodeConfig {
    NodeConfig {
        ftp: Some(FtpConfig {
            address: String::new(),
            port: 21,
            passive_port_min: 30000,
            passive_port_max: 30100,
            public_host: String::new(),
            tls_enabled: false,
            tls_implicit_port: 990,
        }),
        sftp: Some(SftpConfig { port: 2222 }),
    }
}

/// Leftmost match of the Go regex `v?\d+\.\d+\.\d+` (`FindString` keeps the
/// leading `v` when present). Hand-rolled: scan positions left to right; at
/// each, try to read an optional `v` followed by three maximal digit runs
/// separated by dots.
pub fn extract_semver(output: &str) -> Option<String> {
    let bytes = output.as_bytes();
    for start in 0..bytes.len() {
        // Positions inside a digit run can't start a leftmost match that a
        // previous position wouldn't have found longer, but Go's regex engine
        // scans the same way — first position that matches wins.
        if let Some(len) = match_semver_at(bytes, start) {
            return Some(output[start..start + len].to_string());
        }
    }
    None
}

fn match_semver_at(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    if bytes.get(pos) == Some(&b'v') {
        pos += 1;
    }
    for part in 0..3 {
        let run = bytes[pos..].iter().take_while(|b| b.is_ascii_digit()).count();
        if run == 0 {
            return None;
        }
        pos += run;
        if part < 2 {
            if bytes.get(pos) != Some(&b'.') {
                return None;
            }
            pos += 1;
        }
    }
    Some(pos - start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_extraction_table() {
        assert_eq!(
            extract_semver("gameap-files version v1.2.3\nbuild abc").as_deref(),
            Some("v1.2.3")
        );
        assert_eq!(extract_semver("1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(extract_semver("version 10.20.30-rc1").as_deref(), Some("10.20.30"));
        assert_eq!(extract_semver("v1.2"), None);
        assert_eq!(extract_semver(""), None);
        assert_eq!(extract_semver("no version here"), None);
        // Leftmost match wins, like Go's FindString.
        assert_eq!(extract_semver("0.1.2 then 3.4.5").as_deref(), Some("0.1.2"));
    }

    /// Literal JSON as written by the Go plugin (declaration-order fields).
    #[test]
    fn setup_status_round_trip_go_fixtures() {
        let installing =
            r#"{"status":"installing","task_id":7,"last_check":1700000000,"started_at":1700000000}"#;
        let status: NodeSetupStatus = serde_json::from_str(installing).unwrap();
        assert_eq!(status.status, SetupStatus::Installing);
        assert_eq!(status.task_id, 7);
        assert_eq!(status.download_task_id, 0); // absent in legacy docs
        assert_eq!(serde_json::to_string(&status).unwrap(), installing);

        let not_installed = r#"{"status":"not_installed","last_check":1700000000}"#;
        let status: NodeSetupStatus = serde_json::from_str(not_installed).unwrap();
        assert_eq!(serde_json::to_string(&status).unwrap(), not_installed);

        let failed =
            r#"{"status":"failed","task_id":7,"error_message":"boom","last_check":1700000900}"#;
        let status: NodeSetupStatus = serde_json::from_str(failed).unwrap();
        assert_eq!(serde_json::to_string(&status).unwrap(), failed);

        let installed = r#"{"status":"installed","version":"v1.0.0","last_check":1700000000}"#;
        let status: NodeSetupStatus = serde_json::from_str(installed).unwrap();
        assert_eq!(serde_json::to_string(&status).unwrap(), installed);
    }

    #[test]
    fn node_config_round_trip_go_fixtures() {
        let full = concat!(
            r#"{"ftp":{"address":"","port":21,"passive_port_min":30000,"passive_port_max":30100,"#,
            r#""public_host":"","tls_enabled":false,"tls_implicit_port":990},"sftp":{"port":2222}}"#
        );
        let config: NodeConfig = serde_json::from_str(full).unwrap();
        assert_eq!(config, default_node_config());
        assert_eq!(serde_json::to_string(&config).unwrap(), full);

        // Sections are omitempty in the stored form.
        let ftp_only = concat!(
            r#"{"ftp":{"address":"0.0.0.0","port":2121,"passive_port_min":40000,"#,
            r#""passive_port_max":40100,"public_host":"ftp.example.com","tls_enabled":true,"#,
            r#""tls_implicit_port":990}}"#
        );
        let config: NodeConfig = serde_json::from_str(ftp_only).unwrap();
        assert!(config.sftp.is_none());
        assert_eq!(serde_json::to_string(&config).unwrap(), ftp_only);
    }
}
