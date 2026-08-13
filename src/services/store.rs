//! Typed access to the plugin's KV storage. Keys and JSON payloads are
//! byte-compatible with the Go plugin — existing installs must keep working
//! after the swap.

use crate::domain::{FtpUser, NodeConfig, NodeSetupStatus};
use crate::host_api::{HostApi, StorageEntity};
use crate::http::ApiError;

pub const KEY_NODE_SETUP_STATUS: &str = "ftp:setup_status";
pub const KEY_NODE_CONFIG: &str = "ftp:node_config";
pub const KEY_SERVER_USER_LIST: &str = "ftp:users_list";
pub const KEY_USER_PREFIX: &str = "ftp:user:";

pub fn user_key(username: &str) -> String {
    format!("{KEY_USER_PREFIX}{username}")
}

fn parse<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(payload).map_err(|err| ApiError::internal(err.to_string()))
}

fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, ApiError> {
    serde_json::to_vec(value).map_err(|err| ApiError::internal(err.to_string()))
}

pub fn get_user<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
) -> Result<Option<FtpUser>, ApiError> {
    let entity = StorageEntity::server(server_id);
    match host.storage_get(&user_key(username), entity)? {
        Some(payload) => Ok(Some(parse(&payload)?)),
        None => Ok(None),
    }
}

pub fn save_user<H: HostApi>(host: &mut H, server_id: u64, user: &FtpUser) -> Result<(), ApiError> {
    let payload = encode(user)?;
    host.storage_set(
        &user_key(&user.username),
        StorageEntity::server(server_id),
        &payload,
    )?;
    Ok(())
}

pub fn delete_user<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
) -> Result<(), ApiError> {
    host.storage_delete(&user_key(username), StorageEntity::server(server_id))?;
    Ok(())
}

/// All users of a server, from the `ftp:user:` prefix scan. Entries that fail
/// to parse are skipped, matching the Go repository.
pub fn list_users_by_server<H: HostApi>(
    host: &mut H,
    server_id: u64,
) -> Result<Vec<FtpUser>, ApiError> {
    let entries = host.storage_list(KEY_USER_PREFIX, StorageEntity::server(server_id))?;
    Ok(entries
        .iter()
        .filter_map(|(_, payload)| serde_json::from_slice(payload).ok())
        .collect())
}

pub fn get_user_list<H: HostApi>(host: &mut H, server_id: u64) -> Result<Vec<String>, ApiError> {
    match host.storage_get(KEY_SERVER_USER_LIST, StorageEntity::server(server_id))? {
        Some(payload) => parse(&payload),
        None => Ok(Vec::new()),
    }
}

pub fn update_user_list<H: HostApi>(
    host: &mut H,
    server_id: u64,
    usernames: &[String],
) -> Result<(), ApiError> {
    let payload = encode(&usernames)?;
    host.storage_set(
        KEY_SERVER_USER_LIST,
        StorageEntity::server(server_id),
        &payload,
    )?;
    Ok(())
}

pub fn add_to_user_list<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
) -> Result<(), ApiError> {
    let mut usernames = get_user_list(host, server_id)?;
    if usernames.iter().any(|u| u == username) {
        return Ok(());
    }
    usernames.push(username.to_string());
    update_user_list(host, server_id, &usernames)
}

pub fn remove_from_user_list<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
) -> Result<(), ApiError> {
    let usernames: Vec<String> = get_user_list(host, server_id)?
        .into_iter()
        .filter(|u| u != username)
        .collect();
    update_user_list(host, server_id, &usernames)
}

pub fn get_status<H: HostApi>(
    host: &mut H,
    node_id: u64,
) -> Result<Option<NodeSetupStatus>, ApiError> {
    match host.storage_get(KEY_NODE_SETUP_STATUS, StorageEntity::node(node_id))? {
        Some(payload) => Ok(Some(parse(&payload)?)),
        None => Ok(None),
    }
}

pub fn save_status<H: HostApi>(
    host: &mut H,
    node_id: u64,
    status: &NodeSetupStatus,
) -> Result<(), ApiError> {
    let payload = encode(status)?;
    host.storage_set(KEY_NODE_SETUP_STATUS, StorageEntity::node(node_id), &payload)?;
    Ok(())
}

pub fn get_config<H: HostApi>(host: &mut H, node_id: u64) -> Result<Option<NodeConfig>, ApiError> {
    match host.storage_get(KEY_NODE_CONFIG, StorageEntity::node(node_id))? {
        Some(payload) => Ok(Some(parse(&payload)?)),
        None => Ok(None),
    }
}

pub fn save_config<H: HostApi>(
    host: &mut H,
    node_id: u64,
    config: &NodeConfig,
) -> Result<(), ApiError> {
    let payload = encode(config)?;
    host.storage_set(KEY_NODE_CONFIG, StorageEntity::node(node_id), &payload)?;
    Ok(())
}
