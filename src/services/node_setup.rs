//! gameap-files installation orchestration (Go `internal/service/node.go`).
//!
//! Install = two chained CMD_EXEC daemon tasks (`get-tool <script-url>`, then
//! the install script with flags built from [`NodeConfig`]). Progress is
//! tracked in storage; completion arrives either through daemon-task events
//! (matched by task id — see handlers::events) or through the status-poll
//! fallback with its 15-minute timeout.

use crate::domain::{
    NodeConfig, NodeSetupStatus, SetupStatus, default_node_config, extract_semver,
};
use crate::handlers::nodes::NodeSetupRequest;
use crate::host_api::{HostApi, TaskStatus};
use crate::http::ApiError;
use crate::services::{store, yamlpatch};
use crate::shell::shell_join;

pub const INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/gameap/scripts/master/ftp/gameap-files/install-files-linux.sh";
/// If the install task is still "in progress" after this long, mark it failed.
pub const INSTALLING_TIMEOUT_SECS: i64 = 900;
pub const NODE_CONFIG_PATH: &str = "/etc/gameap-files/config.yaml";

pub fn setup_node<H: HostApi>(
    host: &mut H,
    node_id: u64,
    request: &NodeSetupRequest,
) -> Result<NodeSetupStatus, ApiError> {
    if let Some(status) = store::get_status(host, node_id)?
        && status.status == SetupStatus::Installing
    {
        return Ok(status); // Already in progress.
    }

    let node = host
        .get_node(node_id)
        .map_err(|err| ApiError::internal(format!("failed to get node info: {}", err.into_message())))?
        .ok_or_else(|| ApiError::internal(format!("node {node_id} not found")))?;

    let download_cmd = format!("get-tool {INSTALL_SCRIPT_URL}");
    let download_task_id = host
        .create_daemon_task(node_id, &download_cmd, None)
        .map_err(|err| {
            ApiError::internal(format!("failed to create download task: {}", err.into_message()))
        })?;

    let install_cmd = prepare_install_command(&node.work_path, request);
    let install_task_id = host
        .create_daemon_task(node_id, &install_cmd, Some(download_task_id))
        .map_err(|err| {
            ApiError::internal(format!("failed to create install task: {}", err.into_message()))
        })?;

    let now = host.now_unix();
    let new_status = NodeSetupStatus {
        status: SetupStatus::Installing,
        task_id: install_task_id,
        last_check: now,
        started_at: now,
        download_task_id,
        ..NodeSetupStatus::new(SetupStatus::Installing)
    };

    // Both saves are best-effort in Go: log and keep going.
    if let Err(err) = store::save_status(host, node_id, &new_status) {
        host.log_error(&format!("failed to save status: {}", err.message));
    }
    let node_config = merged_node_config(request);
    if let Err(err) = store::save_config(host, node_id, &node_config) {
        host.log_error(&format!("failed to save config: {}", err.message));
    }

    host.log_info(&format!(
        "started gameap-files installation: node_id={node_id} download_task_id={download_task_id} install_task_id={install_task_id}"
    ));

    Ok(new_status)
}

/// Defaults overlaid with whatever the setup request provided
/// (Go `setupConfigToNodeConfig`).
pub fn merged_node_config(request: &NodeSetupRequest) -> NodeConfig {
    let mut config = default_node_config();
    apply_patch(&mut config, request);
    config
}

/// Go `mergeConfigUpdates`: overlay every provided field, creating sections
/// that the target config is missing.
pub fn apply_patch(config: &mut NodeConfig, request: &NodeSetupRequest) {
    if let Some(patch) = &request.ftp {
        let ftp = config.ftp.get_or_insert_with(Default::default);
        if let Some(address) = &patch.address {
            ftp.address = address.clone();
        }
        if let Some(port) = patch.port {
            ftp.port = port;
        }
        if let Some(min) = patch.passive_port_min {
            ftp.passive_port_min = min;
        }
        if let Some(max) = patch.passive_port_max {
            ftp.passive_port_max = max;
        }
        if let Some(public_host) = &patch.public_host {
            ftp.public_host = public_host.clone();
        }
        if let Some(tls_enabled) = patch.tls_enabled {
            ftp.tls_enabled = tls_enabled;
        }
        if let Some(port) = patch.tls_implicit_port {
            ftp.tls_implicit_port = port;
        }
    }
    if let Some(patch) = &request.sftp
        && let Some(port) = patch.port
    {
        config.sftp.get_or_insert_with(Default::default).port = port;
    }
}

/// Builds the install-script invocation. The daemon splits the string with
/// go-shellquote and execs directly, so tokens are joined via shell_join —
/// for ordinary values this yields the same argv as the Go `%q` formatting.
fn prepare_install_command(work_path: &str, request: &NodeSetupRequest) -> String {
    let defaults = default_node_config();
    let mut config = defaults;
    apply_patch(&mut config, request);
    let ftp = config.ftp.unwrap_or_default();
    let sftp = config.sftp.unwrap_or_default();

    let data_dir = format!("--data-dir={work_path}");
    let listen = format!("--ftp-listen-address={}:{}", ftp.address, ftp.port);
    let passive_min = format!("--ftp-passive-port-min={}", ftp.passive_port_min);
    let passive_max = format!("--ftp-passive-port-max={}", ftp.passive_port_max);
    let public_host = format!("--ftp-public-host={}", ftp.public_host);
    let tls_enabled = format!("--ftp-tls-enabled={}", ftp.tls_enabled);
    let tls_port = format!("--ftp-tls-implicit-port=:{}", ftp.tls_implicit_port);
    let sftp_listen = format!("--sftp-listen-address=:{}", sftp.port);

    shell_join(&[
        "install-files-linux.sh",
        &data_dir,
        &listen,
        &passive_min,
        &passive_max,
        &public_host,
        &tls_enabled,
        &tls_port,
        &sftp_listen,
    ])
}

pub fn get_status<H: HostApi>(
    host: &mut H,
    node_id: u64,
) -> Result<Option<NodeSetupStatus>, ApiError> {
    let Some(mut status) = store::get_status(host, node_id)? else {
        // Nothing recorded — probe the node directly (result is not saved).
        return check_installation(host, node_id).map(Some);
    };

    if status.status == SetupStatus::Installing && status.task_id != 0 {
        let now = host.now_unix();

        match check_daemon_task(host, node_id, status.task_id) {
            Err(err) => host.log_warn(&format!("failed to check task status: {}", err.message)),
            Ok(Some(new_status)) => {
                store::save_status(host, node_id, &new_status).ok();
                return Ok(Some(new_status));
            }
            Ok(None) => {}
        }

        // Task still in progress — enforce the timeout. Legacy docs written
        // before started_at existed fall back to last_check.
        let started_at = if status.started_at != 0 {
            status.started_at
        } else {
            status.last_check
        };
        if now - started_at >= INSTALLING_TIMEOUT_SECS {
            host.log_warn(&format!(
                "installation timeout: node_id={node_id} task_id={} elapsed={}s",
                status.task_id,
                now - started_at
            ));
            status = NodeSetupStatus {
                status: SetupStatus::Failed,
                task_id: status.task_id,
                error_message: "installation timed out - event may have been lost".into(),
                last_check: now,
                started_at,
                ..NodeSetupStatus::new(SetupStatus::Failed)
            };
            store::save_status(host, node_id, &status).ok();
        }
    }

    Ok(Some(status))
}

/// Probes `gameap-files version` on the node. The command string is
/// byte-identical to the Go plugin's: the daemon passes `2>/dev/null` and
/// `||` through as literal argv, so the useful signal is the daemon's own
/// "executable file not found" output / non-zero exit — which is exactly what
/// the fallback string checks match.
pub fn check_installation<H: HostApi>(
    host: &mut H,
    node_id: u64,
) -> Result<NodeSetupStatus, ApiError> {
    let resp = host
        .execute_command(node_id, "gameap-files version 2>/dev/null || echo 'not_installed'")
        .map_err(ApiError::from)?;
    let now = host.now_unix();

    let not_installed = NodeSetupStatus {
        status: SetupStatus::NotInstalled,
        last_check: now,
        ..NodeSetupStatus::new(SetupStatus::NotInstalled)
    };

    // Go branched on exit code only, ignoring the response error field.
    if resp.exit_code != 0 {
        host.log_warn(&format!(
            "installation status check exit={} output: {}",
            resp.exit_code, resp.output
        ));
        return Ok(not_installed);
    }

    let output = resp.output.trim();
    let output_lower = output.to_lowercase();
    if output == "not_installed"
        || output_lower.contains("not found")
        || output_lower.contains("no such file")
        || output_lower.contains("executable file")
        || output.is_empty()
    {
        host.log_warn(&format!("installation status output: {}", resp.output));
        return Ok(not_installed);
    }

    match extract_semver(output) {
        Some(version) => Ok(NodeSetupStatus {
            status: SetupStatus::Installed,
            version,
            last_check: now,
            ..NodeSetupStatus::new(SetupStatus::Installed)
        }),
        None => {
            host.log_warn(&format!("installation status output: {}", resp.output));
            Ok(not_installed)
        }
    }
}

/// Polls the tracked install task. `None` = still in progress / not found.
pub fn check_daemon_task<H: HostApi>(
    host: &mut H,
    node_id: u64,
    task_id: u64,
) -> Result<Option<NodeSetupStatus>, ApiError> {
    let Some(task) = host.find_daemon_task(task_id).map_err(ApiError::from)? else {
        return Ok(None);
    };
    match task.status {
        TaskStatus::Success => check_installation(host, node_id).map(Some),
        TaskStatus::Error => Ok(Some(failed_status(host, task_id, task.output))),
        TaskStatus::Canceled => Ok(Some(failed_status(
            host,
            task_id,
            "installation task was canceled".into(),
        ))),
        TaskStatus::Waiting | TaskStatus::Working | TaskStatus::Unknown => Ok(None),
    }
}

fn failed_status<H: HostApi>(host: &mut H, task_id: u64, error_message: String) -> NodeSetupStatus {
    NodeSetupStatus {
        status: SetupStatus::Failed,
        task_id,
        error_message,
        last_check: host.now_unix(),
        ..NodeSetupStatus::new(SetupStatus::Failed)
    }
}

pub fn get_config<H: HostApi>(host: &mut H, node_id: u64) -> Result<NodeConfig, ApiError> {
    Ok(store::get_config(host, node_id)?.unwrap_or_else(default_node_config))
}

pub fn update_config<H: HostApi>(
    host: &mut H,
    node_id: u64,
    config: &NodeConfig,
) -> Result<(), ApiError> {
    store::save_config(host, node_id, config)
        .map_err(|err| ApiError::internal(format!("failed to save config: {}", err.message)))?;
    apply_config_to_node(host, node_id, config).map_err(|err| {
        ApiError::internal(format!("failed to apply config to node: {}", err.message))
    })
}

fn apply_config_to_node<H: HostApi>(
    host: &mut H,
    node_id: u64,
    config: &NodeConfig,
) -> Result<(), ApiError> {
    let original = host
        .download(node_id, NODE_CONFIG_PATH)
        .map_err(|err| ApiError::internal(format!("failed to download config: {}", err.into_message())))?;

    let patched = yamlpatch::patch_config(&original, config).map_err(ApiError::internal)?;

    host.upload(node_id, NODE_CONFIG_PATH, &patched, 0o644)
        .map_err(|err| ApiError::internal(format!("failed to upload config: {}", err.into_message())))?;

    let restart = host
        .execute_command(node_id, "systemctl restart gameap-files.service")
        .map_err(|err| {
            ApiError::internal(format!("failed to restart service: {}", err.into_message()))
        })?;
    if restart.exit_code != 0 {
        return Err(ApiError::internal(format!("restart failed: {}", restart.output)));
    }

    host.log_info(&format!("applied config to node: node_id={node_id}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::nodes::{FtpConfigPatch, SftpConfigPatch};

    #[test]
    fn install_command_with_defaults() {
        let cmd = prepare_install_command("/srv/gameap", &NodeSetupRequest::default());
        assert_eq!(
            cmd,
            "install-files-linux.sh --data-dir=/srv/gameap --ftp-listen-address=:21 \
             --ftp-passive-port-min=30000 --ftp-passive-port-max=30100 --ftp-public-host= \
             --ftp-tls-enabled=false --ftp-tls-implicit-port=:990 --sftp-listen-address=:2222"
        );
    }

    #[test]
    fn install_command_with_overrides() {
        let request = NodeSetupRequest {
            ftp: Some(FtpConfigPatch {
                address: Some("0.0.0.0".into()),
                port: Some(2121),
                passive_port_min: Some(40000),
                passive_port_max: Some(40100),
                public_host: Some("ftp.example.com".into()),
                tls_enabled: Some(true),
                tls_implicit_port: Some(991),
            }),
            sftp: Some(SftpConfigPatch { port: Some(2223) }),
        };
        let cmd = prepare_install_command("/srv/game ap", &request);
        assert!(cmd.contains("'--data-dir=/srv/game ap'"));
        assert!(cmd.contains("--ftp-listen-address=0.0.0.0:2121"));
        assert!(cmd.contains("--ftp-passive-port-min=40000"));
        assert!(cmd.contains("--ftp-public-host=ftp.example.com"));
        assert!(cmd.contains("--ftp-tls-enabled=true"));
        assert!(cmd.contains("--ftp-tls-implicit-port=:991"));
        assert!(cmd.contains("--sftp-listen-address=:2223"));
    }
}
