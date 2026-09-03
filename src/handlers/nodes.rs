//! Node setup/status/config endpoints (Go `internal/handlers/node.go`).

use serde::{Deserialize, Serialize};

use crate::domain::{NodeConfig, NodeSetupStatus};
use crate::host_api::HostApi;
use crate::http::{ApiResult, json_response, parse_json_body_verbose, parse_u64_param};
use crate::router::RequestParts;
use crate::services::node_setup;

/// HTTP status DTO. Unlike the stored `NodeSetupStatus`, `last_check` is
/// omitempty here and `started_at`/`download_task_id` are never exposed —
/// both quirks inherited from the Go DTO.
#[derive(Debug, Serialize)]
pub struct NodeStatusResponse {
    pub status: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub task_id: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error_message: String,
    #[serde(skip_serializing_if = "is_zero_i64")]
    pub last_check: i64,
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

impl NodeStatusResponse {
    pub fn not_installed() -> Self {
        Self {
            status: "not_installed",
            version: String::new(),
            task_id: 0,
            error_message: String::new(),
            last_check: 0,
        }
    }
}

impl From<&NodeSetupStatus> for NodeStatusResponse {
    fn from(status: &NodeSetupStatus) -> Self {
        Self {
            status: status.status.as_str(),
            version: status.version.clone(),
            task_id: status.task_id,
            error_message: status.error_message.clone(),
            last_check: status.last_check,
        }
    }
}

/// Setup and config-update requests share one shape: every field optional.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct NodeSetupRequest {
    pub ftp: Option<FtpConfigPatch>,
    pub sftp: Option<SftpConfigPatch>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FtpConfigPatch {
    pub address: Option<String>,
    pub port: Option<i64>,
    pub passive_port_min: Option<i64>,
    pub passive_port_max: Option<i64>,
    pub public_host: Option<String>,
    pub tls_enabled: Option<bool>,
    pub tls_implicit_port: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SftpConfigPatch {
    pub port: Option<i64>,
}

/// Config response: Go serialized both section pointers without omitempty, so
/// an absent section is an explicit `null`.
#[derive(Debug, Serialize)]
struct NodeConfigResponse<'a> {
    ftp: Option<&'a crate::domain::FtpConfig>,
    sftp: Option<&'a crate::domain::SftpConfig>,
}

impl<'a> From<&'a NodeConfig> for NodeConfigResponse<'a> {
    fn from(config: &'a NodeConfig) -> Self {
        Self {
            ftp: config.ftp.as_ref(),
            sftp: config.sftp.as_ref(),
        }
    }
}

pub fn setup<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let node_id = parse_u64_param(&parts.params, "nodeId")?;

    // An empty body keeps the node's stored configuration (defaults when
    // nothing is stored yet), which is what the Update button relies on.
    let request: NodeSetupRequest = if parts.body.is_empty() {
        NodeSetupRequest::default()
    } else {
        parse_json_body_verbose(parts.body)?
    };

    let status = node_setup::setup_node(host, node_id, &request)?;
    Ok(json_response(200, &NodeStatusResponse::from(&status)))
}

pub fn status<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let node_id = parse_u64_param(&parts.params, "nodeId")?;
    let response = match node_setup::get_status(host, node_id)? {
        Some(status) => NodeStatusResponse::from(&status),
        None => NodeStatusResponse::not_installed(),
    };
    Ok(json_response(200, &response))
}

pub fn get_config<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let node_id = parse_u64_param(&parts.params, "nodeId")?;
    let config = node_setup::get_config(host, node_id)?;
    Ok(json_response(200, &NodeConfigResponse::from(&config)))
}

pub fn update_config<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let node_id = parse_u64_param(&parts.params, "nodeId")?;
    let request: NodeSetupRequest = parse_json_body_verbose(parts.body)?;

    let mut config = node_setup::get_config(host, node_id)?;
    node_setup::apply_patch(&mut config, &request);
    node_setup::update_config(host, node_id, &config)?;

    Ok(json_response(200, &NodeConfigResponse::from(&config)))
}
