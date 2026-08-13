//! FTP user endpoints (Go `internal/handlers/user.go`). DTO field names and
//! order, status codes and error messages are wire-compatible.

use serde::{Deserialize, Serialize};

use crate::domain::{AccessRule, FtpUser, VirtualPath, null_to_default};
use crate::host_api::HostApi;
use crate::http::{
    ApiResult, get_string_param, json_response, parse_int_param, parse_json_body, parse_u64_param,
};
use crate::router::RequestParts;
use crate::services::{sync, users};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CreateUserRequest {
    #[serde(deserialize_with = "null_to_default")]
    pub username: String,
    pub password: Option<String>,
    pub home_dir: Option<String>,
    #[serde(deserialize_with = "null_to_default")]
    pub quota_bytes: i64,
    pub enabled: Option<bool>,
    #[serde(deserialize_with = "null_to_default")]
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub home_dir: String,
    pub quota_bytes: i64,
    pub enabled: bool,
    pub description: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateUserRequest {
    pub password: Option<String>,
    pub home_dir: Option<String>,
    pub quota_bytes: Option<i64>,
    pub enabled: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub username: String,
    pub home_dir: String,
    pub quota_bytes: i64,
    pub enabled: bool,
    pub description: String,
    pub ssh_keys_count: usize,
    pub access_rules: Vec<AccessRule>,
    pub virtual_paths: Vec<VirtualPath>,
}

impl From<FtpUser> for UserResponse {
    fn from(user: FtpUser) -> Self {
        Self {
            username: user.username,
            home_dir: user.home_dir,
            quota_bytes: user.quota_bytes,
            enabled: user.enabled,
            description: user.description,
            ssh_keys_count: user.ssh_public_keys.len(),
            access_rules: user.access_rules,
            virtual_paths: user.virtual_paths,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AccessRulesRequest {
    #[serde(deserialize_with = "null_to_default")]
    rules: Vec<AccessRule>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct VirtualPathsRequest {
    #[serde(deserialize_with = "null_to_default")]
    paths: Vec<VirtualPath>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AddSshKeyRequest {
    #[serde(deserialize_with = "null_to_default")]
    key: String,
}

#[derive(Debug, Serialize)]
struct SshKeysResponse {
    keys: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DeletedResponse {
    deleted: bool,
}

fn warn_sync_failure<H: HostApi>(host: &mut H, err: crate::http::ApiError) {
    host.log_warn(&format!("failed to sync user to node: {}", err.message));
}

pub fn list<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let users = users::list(host, server_id)?;
    let response: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(json_response(200, &response))
}

pub fn create<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let input: CreateUserRequest = parse_json_body(parts.body)?;

    if input.username.is_empty() {
        return Err(crate::http::ApiError::bad_request("username is required"));
    }

    // Go resolved the server before creating (a missing server is a 500) and
    // used its dir as the default home. One lookup serves both here.
    let server = host
        .get_server(server_id)?
        .ok_or_else(|| crate::http::ApiError::internal(format!("server {server_id} not found")))?;
    let home_dir = match input.home_dir.as_deref() {
        Some(dir) if !dir.is_empty() => dir.to_string(),
        _ => server.dir,
    };

    let result = users::create(
        host,
        server_id,
        users::CreateUserInput {
            username: input.username,
            password: input.password,
            home_dir,
            quota_bytes: input.quota_bytes,
            enabled: input.enabled.unwrap_or(true),
            description: input.description,
        },
    )?;

    if let Err(err) = sync::sync_user_to_node(host, server_id, &result.user) {
        warn_sync_failure(host, err);
    }

    Ok(json_response(
        201,
        &CreateUserResponse {
            username: result.user.username,
            password: result.password,
            home_dir: result.user.home_dir,
            quota_bytes: result.user.quota_bytes,
            enabled: result.user.enabled,
            description: result.user.description,
        },
    ))
}

pub fn get<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?;
    let user = users::get(host, server_id, username)?;
    Ok(json_response(200, &UserResponse::from(user)))
}

pub fn update<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?.to_string();
    let input: UpdateUserRequest = parse_json_body(parts.body)?;

    // Go resolved the server's node before updating; a missing server is a
    // 500 even though the id itself went unused.
    sync::node_id_for_server(host, server_id)?;

    let user = users::update(
        host,
        server_id,
        &username,
        users::UpdateUserInput {
            password: input.password,
            home_dir: input.home_dir,
            quota_bytes: input.quota_bytes,
            enabled: input.enabled,
            description: input.description,
        },
    )?;

    if let Err(err) = sync::sync_user_to_node(host, server_id, &user) {
        warn_sync_failure(host, err);
    }

    Ok(json_response(200, &UserResponse::from(user)))
}

pub fn delete<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?.to_string();

    users::delete(host, server_id, &username)?;

    if let Err(err) = sync::delete_user_from_node(host, server_id, &username) {
        host.log_warn(&format!("failed to delete user from node: {}", err.message));
    }

    Ok(json_response(200, &DeletedResponse { deleted: true }))
}

#[derive(Debug, Serialize)]
struct RulesResponse {
    rules: Vec<AccessRule>,
}

pub fn get_access_rules<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?;
    let user = users::get(host, server_id, username)?;
    Ok(json_response(200, &RulesResponse { rules: user.access_rules }))
}

pub fn update_access_rules<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?.to_string();
    let input: AccessRulesRequest = parse_json_body(parts.body)?;

    let user = users::update_access_rules(host, server_id, &username, input.rules)?;

    if let Err(err) = sync::sync_user_to_node(host, server_id, &user) {
        warn_sync_failure(host, err);
    }

    Ok(json_response(200, &RulesResponse { rules: user.access_rules }))
}

#[derive(Debug, Serialize)]
struct PathsResponse {
    paths: Vec<VirtualPath>,
}

pub fn get_virtual_paths<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?;
    let user = users::get(host, server_id, username)?;
    Ok(json_response(200, &PathsResponse { paths: user.virtual_paths }))
}

pub fn update_virtual_paths<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?.to_string();
    let input: VirtualPathsRequest = parse_json_body(parts.body)?;

    let user = users::update_virtual_paths(host, server_id, &username, input.paths)?;

    if let Err(err) = sync::sync_user_to_node(host, server_id, &user) {
        warn_sync_failure(host, err);
    }

    Ok(json_response(200, &PathsResponse { paths: user.virtual_paths }))
}

pub fn list_ssh_keys<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?;
    let user = users::get(host, server_id, username)?;
    Ok(json_response(200, &SshKeysResponse { keys: user.ssh_public_keys }))
}

pub fn add_ssh_key<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?.to_string();
    let input: AddSshKeyRequest = parse_json_body(parts.body)?;
    if input.key.is_empty() {
        return Err(crate::http::ApiError::bad_request("key is required"));
    }

    let user = users::add_ssh_key(host, server_id, &username, input.key)?;

    if let Err(err) = sync::sync_user_to_node(host, server_id, &user) {
        warn_sync_failure(host, err);
    }

    Ok(json_response(201, &SshKeysResponse { keys: user.ssh_public_keys }))
}

pub fn delete_ssh_key<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let server_id = parse_u64_param(&parts.params, "serverId")?;
    let username = get_string_param(&parts.params, "username")?.to_string();
    let index = parse_int_param(&parts.params, "index")?;

    let user = users::remove_ssh_key(host, server_id, &username, index)?;

    if let Err(err) = sync::sync_user_to_node(host, server_id, &user) {
        warn_sync_failure(host, err);
    }

    Ok(json_response(200, &SshKeysResponse { keys: user.ssh_public_keys }))
}
