//! Mirrors storage state to nodes: one YAML drop-in per user under
//! `/etc/gameap-files/users.d/` (the daemon hot-reloads the directory).
//! All sync failures are the caller's to downgrade — handlers log a warning
//! and keep the HTTP request successful, exactly like the Go plugin.

use crate::domain::FtpUser;
use crate::host_api::HostApi;
use crate::http::ApiError;

pub const USERS_CONFIG_DIR: &str = "/etc/gameap-files/users.d";
const USER_CONFIG_PERM: u32 = 0o600;

pub fn user_config_path(username: &str) -> String {
    format!("{USERS_CONFIG_DIR}/{username}.yaml")
}

/// Node id of a server, with the Go plugin's "server %d not found" message
/// (surfaces as a 500, which the create/update handlers rely on).
pub fn node_id_for_server<H: HostApi>(host: &mut H, server_id: u64) -> Result<u64, ApiError> {
    let server = host
        .get_server(server_id)?
        .ok_or_else(|| ApiError::internal(format!("server {server_id} not found")))?;
    Ok(server.node_id)
}

pub fn server_dir<H: HostApi>(host: &mut H, server_id: u64) -> Result<String, ApiError> {
    let server = host
        .get_server(server_id)?
        .ok_or_else(|| ApiError::internal(format!("server {server_id} not found")))?;
    Ok(server.dir)
}

pub fn sync_user_to_node<H: HostApi>(
    host: &mut H,
    server_id: u64,
    user: &FtpUser,
) -> Result<(), ApiError> {
    let node_id = node_id_for_server(host, server_id)
        .map_err(|err| ApiError::internal(format!("failed to get node for server: {}", err.message)))?;
    upload_user_config(host, node_id, user)
}

fn upload_user_config<H: HostApi>(
    host: &mut H,
    node_id: u64,
    user: &FtpUser,
) -> Result<(), ApiError> {
    let yaml = serde_yaml_ng::to_string(user)
        .map_err(|err| ApiError::internal(format!("failed to serialize user config: {err}")))?;
    host.upload(
        node_id,
        &user_config_path(&user.username),
        yaml.as_bytes(),
        USER_CONFIG_PERM,
    )
    .map_err(|err| ApiError::internal(format!("failed to upload user config: {}", err.into_message())))?;
    host.log_info(&format!(
        "synced user to node: username={} node_id={node_id}",
        user.username
    ));
    Ok(())
}

/// Removes a user's YAML from the node. A daemon-reported failure (e.g. file
/// already absent) is only warned about — Go treated `!resp.Success` the same.
pub fn delete_user_from_node<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
) -> Result<(), ApiError> {
    let node_id = node_id_for_server(host, server_id)
        .map_err(|err| ApiError::internal(format!("failed to get node for server: {}", err.message)))?;
    remove_user_config(host, node_id, username);
    Ok(())
}

fn remove_user_config<H: HostApi>(host: &mut H, node_id: u64, username: &str) {
    use crate::host_api::HostApiError;
    match host.remove(node_id, &user_config_path(username), false) {
        Ok(()) => {}
        Err(HostApiError::Op(message)) => {
            if !message.is_empty() {
                host.log_warn(&format!("remove user config warning: {message}"));
            }
        }
        Err(HostApiError::Call(message)) => {
            host.log_warn(&format!("failed to remove user config: {message}"));
        }
    }
    host.log_info(&format!(
        "deleted user from node: username={username} node_id={node_id}"
    ));
}

/// Removes all listed users' YAML files from a node. Takes the node id
/// directly: on SERVER_DELETED the server row is already gone, so resolving
/// it via get_server (as Go did) can no longer work — the event payload's
/// ds_id is used instead.
pub fn delete_all_users_from_node<H: HostApi>(
    host: &mut H,
    node_id: u64,
    usernames: &[String],
) {
    for username in usernames {
        remove_user_config(host, node_id, username);
    }
}
