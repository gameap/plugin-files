//! gameap-files installation orchestration (Go `internal/service/node.go`).
//!
//! Install = two chained CMD_EXEC daemon tasks (`get-tool <script-url>`, then
//! the install script with flags built from [`NodeConfig`]). Progress is
//! tracked in storage; completion arrives either through daemon-task events
//! (matched by task id — see handlers::events) or through the status-poll
//! fallback with its 15-minute timeout. Whichever observes the completion
//! first also re-syncs the node's users ([`complete_installation`]).
//!
//! Linux nodes run the bash installer; Windows nodes run the PowerShell one.
//! Windows command strings are built with [`shell_join_windows`] and address
//! the script through the daemon's `{node_tools_path}` / `{node_work_path}`
//! placeholders, which the daemon substitutes before splitting a task command.

use crate::domain::{
    FtpConfig, NodeConfig, NodeOs, NodeSetupStatus, SetupStatus, SftpConfig,
    default_node_config, extract_semver,
};
use crate::handlers::nodes::NodeSetupRequest;
use crate::host_api::{CommandOutput, HostApi, NodeInfo, TaskStatus};
use crate::http::ApiError;
use crate::services::{store, sync, yamlpatch};
use crate::shell::{shell_join, shell_join_windows};

pub const INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/gameap/scripts/master/ftp/gameap-files/install-files-linux.sh";
pub const WINDOWS_INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/gameap/scripts/master/ftp/gameap-files/install-files-windows.ps1";
const WINDOWS_INSTALL_SCRIPT_NAME: &str = "install-files-windows.ps1";
/// If the install task is still "in progress" after this long, mark it failed.
pub const INSTALLING_TIMEOUT_SECS: i64 = 900;
/// Relative to the node work path, like every path handed to the daemon.
pub const NODE_CONFIG_PATH: &str = ".plugins/files/config.yaml";
pub const WINDOWS_SERVICE_NAME: &str = "gameap-files";
/// Byte-identical to the Go plugin's: the daemon passes `2>/dev/null` and
/// `||` through as literal argv, so the useful signal is the daemon's own
/// "executable file not found" output / non-zero exit.
const LINUX_VERSION_PROBE_CMD: &str = "gameap-files version 2>/dev/null || echo 'not_installed'";
/// The Windows installer is pinned to `<tools>\gameap-files`, a directory that
/// does not exist when `get-tool` refreshes the daemon PATH, so the probe has
/// to name the binary outright.
const WINDOWS_BINARY_REL_PATH: &str = r"tools\gameap-files\gameap-files.exe";
pub const LINUX_RESTART_SYSTEM_CMD: &str = "systemctl restart gameap-files.service";
/// A rootless gameap-daemon runs gameap-files as a user unit; `systemctl
/// --user` needs the user manager's runtime directory, which a daemon started
/// without a session does not have in its environment.
pub const LINUX_RESTART_USER_CMD: &str = "sh -c 'XDG_RUNTIME_DIR=\"${XDG_RUNTIME_DIR:-/run/user/$(id -u)}\" systemctl --user restart gameap-files.service'";
/// `-ErrorAction Stop`: a missing service is otherwise a non-terminating
/// error and `powershell -Command` exits 0 regardless.
pub const WINDOWS_RESTART_CMD: &str = "powershell -NoProfile -NonInteractive -Command \"Restart-Service -Name gameap-files -ErrorAction Stop\"";

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

    let node = get_node(host, node_id)?;
    let node_config = setup_config(host, node_id, request)?;
    // Refused before any task exists, so an unsupported node never ends up
    // "installing".
    let install_cmd = prepare_install_command(&node, &node_config)?;

    let download_cmd = format!("get-tool {}", install_script_url(node.os_kind()));
    let download_task_id = host
        .create_daemon_task(node_id, &download_cmd, None)
        .map_err(|err| {
            ApiError::internal(format!("failed to create download task: {}", err.into_message()))
        })?;

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
    if let Err(err) = store::save_config(host, node_id, &node_config) {
        host.log_error(&format!("failed to save config: {}", err.message));
    }

    host.log_info(&format!(
        "started gameap-files installation: node_id={node_id} os={} download_task_id={download_task_id} install_task_id={install_task_id}",
        node.os
    ));

    Ok(new_status)
}

fn get_node<H: HostApi>(host: &mut H, node_id: u64) -> Result<NodeInfo, ApiError> {
    host.get_node(node_id)
        .map_err(|err| ApiError::internal(format!("failed to get node info: {}", err.into_message())))?
        .ok_or_else(|| ApiError::internal(format!("node {node_id} not found")))
}

/// The node's stored configuration (defaults when nothing is stored) overlaid
/// with whatever the setup request provided. Re-running setup on an installed
/// node — the Update button sends no body — therefore keeps its ports; the Go
/// `setupConfigToNodeConfig` started from the defaults every time.
pub fn setup_config<H: HostApi>(
    host: &mut H,
    node_id: u64,
    request: &NodeSetupRequest,
) -> Result<NodeConfig, ApiError> {
    let mut config = get_config(host, node_id)?;
    complete_config(&mut config);
    apply_patch(&mut config, request);
    Ok(config)
}

/// Fills sections a stored document lacks from the defaults, so a command
/// built from it never sees a zero port.
fn complete_config(config: &mut NodeConfig) {
    let defaults = default_node_config();
    if config.ftp.is_none() {
        config.ftp = defaults.ftp;
    }
    if config.sftp.is_none() {
        config.sftp = defaults.sftp;
    }
}

fn effective_sections(config: &NodeConfig) -> (FtpConfig, SftpConfig) {
    let mut config = config.clone();
    complete_config(&mut config);
    (
        config.ftp.unwrap_or_default(),
        config.sftp.unwrap_or_default(),
    )
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

fn install_script_url(os: NodeOs) -> &'static str {
    match os {
        NodeOs::Windows => WINDOWS_INSTALL_SCRIPT_URL,
        NodeOs::Linux | NodeOs::Unsupported => INSTALL_SCRIPT_URL,
    }
}

/// Builds the install-script invocation for the node's OS. Only Linux and
/// Windows nodes have an installer; anything else is a 400 for the caller.
pub fn prepare_install_command(node: &NodeInfo, config: &NodeConfig) -> Result<String, ApiError> {
    match node.os_kind() {
        NodeOs::Linux => Ok(linux_install_command(&node.work_path, config)),
        NodeOs::Windows => Ok(windows_install_command(config)),
        NodeOs::Unsupported => Err(ApiError::bad_request(format!(
            "unsupported node OS \"{}\": gameap-files can be installed on linux and windows nodes only",
            node.os
        ))),
    }
}

/// The daemon splits the string with go-shellquote and execs directly, so
/// tokens are joined via shell_join — for ordinary values this yields the
/// same argv as the Go `%q` formatting.
fn linux_install_command(work_path: &str, config: &NodeConfig) -> String {
    let (ftp, sftp) = effective_sections(config);

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

/// The script path must match where `get-tool` put the file, which is the
/// daemon's own tools directory — hence the placeholder rather than the work
/// path the panel knows. `-InstallDir` is pinned below that same directory so
/// the version probe can name the binary; `-DataDir` is the daemon work path,
/// which makes `<DataDir>\.plugins\files` exactly where this plugin's uploads
/// land. `-File` stays last among the PowerShell options: everything after it
/// belongs to the script.
fn windows_install_command(config: &NodeConfig) -> String {
    let (ftp, sftp) = effective_sections(config);

    let script = format!("{{node_tools_path}}/{WINDOWS_INSTALL_SCRIPT_NAME}");
    let install_dir = format!("{{node_tools_path}}/{WINDOWS_SERVICE_NAME}");
    let listen = format!("{}:{}", ftp.address, ftp.port);
    let passive_min = ftp.passive_port_min.to_string();
    let passive_max = ftp.passive_port_max.to_string();
    let tls_port = format!(":{}", ftp.tls_implicit_port);
    let sftp_listen = format!(":{}", sftp.port);

    let mut args: Vec<&str> = vec![
        "powershell",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &script,
        "-DataDir",
        "{node_work_path}",
        "-InstallDir",
        &install_dir,
        "-FtpListenAddress",
        &listen,
        "-FtpPassivePortMin",
        &passive_min,
        "-FtpPassivePortMax",
        &passive_max,
        "-FtpTlsImplicitPort",
        &tls_port,
        "-SftpListenAddress",
        &sftp_listen,
    ];
    if !ftp.public_host.is_empty() {
        args.push("-FtpPublicHost");
        args.push(&ftp.public_host);
    }
    if ftp.tls_enabled {
        args.push("-FtpTlsEnabled");
    }

    shell_join_windows(&args)
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
                return Ok(Some(complete_installation(host, node_id, new_status)));
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

/// Persists the outcome of an installation. A successful one also pushes
/// every user of the node to the users directory and sweeps the misplaced
/// files of earlier releases — once: the flag guards against the event and
/// the status poll both observing the same completion, or two panel
/// instances sharing one storage.
pub fn complete_installation<H: HostApi>(
    host: &mut H,
    node_id: u64,
    mut status: NodeSetupStatus,
) -> NodeSetupStatus {
    if status.status == SetupStatus::Installed && !status.synced_after_install {
        // Recorded before the resync so its outcome survives a resync that
        // runs out of budget.
        if let Err(err) = store::save_status(host, node_id, &status) {
            host.log_error(&format!("failed to save status: {}", err.message));
        }
        let report = sync::resync_node_users(host, node_id);
        host.log_info(&format!(
            "post-install resync: node_id={node_id} synced={} failed={} legacy_removed={}",
            report.synced, report.failed, report.legacy_removed
        ));
        status.synced_after_install = true;
    }
    if let Err(err) = store::save_status(host, node_id, &status) {
        host.log_error(&format!("failed to save status: {}", err.message));
    }
    status
}

/// Probes `gameap-files version` on the node.
pub fn check_installation<H: HostApi>(
    host: &mut H,
    node_id: u64,
) -> Result<NodeSetupStatus, ApiError> {
    let node = host.get_node(node_id).ok().flatten();
    let probe = version_probe_command(node.as_ref());
    let resp = host
        .execute_command(node_id, &probe)
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

fn version_probe_command(node: Option<&NodeInfo>) -> String {
    match node {
        Some(node) if node.os_kind() == NodeOs::Windows => {
            let binary = sync::join_node_path(&node.work_path, WINDOWS_BINARY_REL_PATH);
            shell_join_windows(&[&binary, "version"])
        }
        _ => LINUX_VERSION_PROBE_CMD.to_string(),
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
    let node = get_node(host, node_id)?;

    let original = host
        .download(node_id, NODE_CONFIG_PATH)
        .map_err(|err| ApiError::internal(format!("failed to download config: {}", err.into_message())))?;

    let patched = yamlpatch::patch_config(&original, config).map_err(ApiError::internal)?;

    host.upload(node_id, NODE_CONFIG_PATH, &patched, 0o644)
        .map_err(|err| ApiError::internal(format!("failed to upload config: {}", err.into_message())))?;

    restart_service(host, &node)?;

    host.log_info(&format!("applied config to node: node_id={node_id}"));
    Ok(())
}

/// Windows restarts the service; Linux tries the system unit first and falls
/// back to the user unit of a rootless daemon.
fn restart_service<H: HostApi>(host: &mut H, node: &NodeInfo) -> Result<(), ApiError> {
    if node.os_kind() == NodeOs::Windows {
        let restart = run_restart(host, node.id, WINDOWS_RESTART_CMD)?;
        if restart.exit_code != 0 {
            return Err(ApiError::internal(format!(
                "restart failed: {}",
                restart.output.trim()
            )));
        }
        return Ok(());
    }

    let system = run_restart(host, node.id, LINUX_RESTART_SYSTEM_CMD)?;
    if system.exit_code == 0 {
        return Ok(());
    }
    host.log_warn(&format!(
        "system unit restart failed on node {} (exit {}), trying the user unit: {}",
        node.id,
        system.exit_code,
        system.output.trim()
    ));

    let user = run_restart(host, node.id, LINUX_RESTART_USER_CMD)?;
    if user.exit_code == 0 {
        return Ok(());
    }
    Err(ApiError::internal(format!(
        "restart failed: system unit: {}; user unit: {}",
        system.output.trim(),
        user.output.trim()
    )))
}

fn run_restart<H: HostApi>(
    host: &mut H,
    node_id: u64,
    command: &str,
) -> Result<CommandOutput, ApiError> {
    host.execute_command(node_id, command).map_err(|err| {
        ApiError::internal(format!("failed to restart service: {}", err.into_message()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::nodes::{FtpConfigPatch, SftpConfigPatch};

    fn node(os: &str, work_path: &str) -> NodeInfo {
        NodeInfo {
            id: 1,
            name: "n".into(),
            ips: vec![],
            work_path: work_path.into(),
            os: os.into(),
        }
    }

    fn overrides() -> NodeConfig {
        let mut config = default_node_config();
        apply_patch(
            &mut config,
            &NodeSetupRequest {
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
            },
        );
        config
    }

    #[test]
    fn install_command_with_defaults() {
        let cmd =
            prepare_install_command(&node("linux", "/srv/gameap"), &default_node_config()).unwrap();
        assert_eq!(
            cmd,
            "install-files-linux.sh --data-dir=/srv/gameap --ftp-listen-address=:21 \
             --ftp-passive-port-min=30000 --ftp-passive-port-max=30100 --ftp-public-host= \
             --ftp-tls-enabled=false --ftp-tls-implicit-port=:990 --sftp-listen-address=:2222"
        );
    }

    #[test]
    fn install_command_with_overrides() {
        let cmd = prepare_install_command(&node("linux", "/srv/game ap"), &overrides()).unwrap();
        assert!(cmd.contains("'--data-dir=/srv/game ap'"));
        assert!(cmd.contains("--ftp-listen-address=0.0.0.0:2121"));
        assert!(cmd.contains("--ftp-passive-port-min=40000"));
        assert!(cmd.contains("--ftp-public-host=ftp.example.com"));
        assert!(cmd.contains("--ftp-tls-enabled=true"));
        assert!(cmd.contains("--ftp-tls-implicit-port=:991"));
        assert!(cmd.contains("--sftp-listen-address=:2223"));
    }

    #[test]
    fn install_command_fills_missing_sections_from_defaults() {
        let config = NodeConfig {
            ftp: None,
            sftp: None,
        };
        let cmd = prepare_install_command(&node("", "/srv/gameap"), &config).unwrap();
        assert!(cmd.contains("--ftp-listen-address=:21"), "{cmd}");
        assert!(cmd.contains("--sftp-listen-address=:2222"), "{cmd}");
    }

    #[test]
    fn windows_install_command_with_defaults() {
        let cmd =
            prepare_install_command(&node("windows", r"C:\gameap"), &default_node_config()).unwrap();
        assert_eq!(
            cmd,
            "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass \
             -File \"{node_tools_path}/install-files-windows.ps1\" \
             -DataDir \"{node_work_path}\" -InstallDir \"{node_tools_path}/gameap-files\" \
             -FtpListenAddress :21 -FtpPassivePortMin 30000 -FtpPassivePortMax 30100 \
             -FtpTlsImplicitPort :990 -SftpListenAddress :2222"
        );
    }

    #[test]
    fn windows_install_command_with_overrides() {
        let cmd = prepare_install_command(&node("windows", r"C:\gameap"), &overrides()).unwrap();
        assert!(cmd.contains("-FtpListenAddress 0.0.0.0:2121"), "{cmd}");
        assert!(cmd.contains("-FtpPassivePortMin 40000"), "{cmd}");
        assert!(cmd.contains("-FtpPassivePortMax 40100"), "{cmd}");
        assert!(cmd.contains("-FtpTlsImplicitPort :991"), "{cmd}");
        assert!(cmd.contains("-SftpListenAddress :2223"), "{cmd}");
        assert!(cmd.contains("-FtpPublicHost ftp.example.com"), "{cmd}");
        assert!(cmd.ends_with(" -FtpTlsEnabled"), "{cmd}");
    }

    #[test]
    fn unsupported_os_is_rejected() {
        let err = prepare_install_command(&node("macos", "/Users/gameap"), &default_node_config())
            .unwrap_err();
        assert_eq!(err.status, 400);
        assert!(err.message.starts_with("unsupported node OS \"macos\""), "{}", err.message);
    }

    #[test]
    fn install_script_url_per_os() {
        assert_eq!(install_script_url(NodeOs::Linux), INSTALL_SCRIPT_URL);
        assert_eq!(install_script_url(NodeOs::Windows), WINDOWS_INSTALL_SCRIPT_URL);
    }

    #[test]
    fn version_probe_per_os() {
        assert_eq!(version_probe_command(None), LINUX_VERSION_PROBE_CMD);
        assert_eq!(
            version_probe_command(Some(&node("linux", "/srv/gameap"))),
            LINUX_VERSION_PROBE_CMD
        );
        assert_eq!(
            version_probe_command(Some(&node("windows", r"C:\gameap"))),
            r"C:\gameap\tools\gameap-files\gameap-files.exe version"
        );
        assert_eq!(
            version_probe_command(Some(&node("windows", r"C:\Program Files\gameap"))),
            r#""C:\Program Files\gameap\tools\gameap-files\gameap-files.exe" version"#
        );
    }
}
