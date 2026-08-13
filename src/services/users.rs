//! FTP user business logic (Go `internal/service/user.go`). Error statuses,
//! codes and messages replicate the Go service + handler mapping exactly.

use crate::domain::{self, FtpUser, VirtualPath};
use crate::domain::user::AccessRule;
use crate::host_api::HostApi;
use crate::http::ApiError;
use crate::services::{password, store};

pub const ERR_INVALID_USERNAME: &str =
    "invalid username: must be alphanumeric with underscores, 3-32 chars";
pub const ERR_INVALID_SSH_KEY: &str = "invalid SSH public key format";

fn user_not_found() -> ApiError {
    ApiError::not_found("user not found")
}

pub struct CreateUserInput {
    pub username: String,
    pub password: Option<String>,
    pub home_dir: String,
    pub quota_bytes: i64,
    pub enabled: bool,
    pub description: String,
}

pub struct CreateUserResult {
    pub user: FtpUser,
    /// Set only when the password was auto-generated.
    pub password: Option<String>,
}

#[derive(Default)]
pub struct UpdateUserInput {
    pub password: Option<String>,
    pub home_dir: Option<String>,
    pub quota_bytes: Option<i64>,
    pub enabled: Option<bool>,
    pub description: Option<String>,
}

pub fn create<H: HostApi>(
    host: &mut H,
    server_id: u64,
    input: CreateUserInput,
) -> Result<CreateUserResult, ApiError> {
    if !domain::validate_username(&input.username) {
        return Err(ApiError::bad_request(ERR_INVALID_USERNAME));
    }

    if store::get_user(host, server_id, &input.username)?.is_some() {
        return Err(ApiError::conflict("user already exists"));
    }

    let mut plain_password = None;
    let password_hash = match input.password.as_deref() {
        Some(provided) if !provided.is_empty() => password::hash_password(host, provided)?,
        _ => {
            let generated = password::generate_password(host)?;
            let hash = password::hash_password(host, &generated)?;
            plain_password = Some(generated);
            hash
        }
    };

    let user = FtpUser {
        username: input.username,
        password_hash,
        ssh_public_keys: Vec::new(),
        home_dir: input.home_dir,
        quota_bytes: input.quota_bytes,
        access_rules: domain::default_access_rules(),
        virtual_paths: Vec::new(),
        enabled: input.enabled,
        description: input.description,
    };

    store::save_user(host, server_id, &user)?;

    if let Err(err) = store::add_to_user_list(host, server_id, &user.username) {
        host.log_warn(&format!("failed to update user list: {}", err.message));
    }

    Ok(CreateUserResult {
        user,
        password: plain_password,
    })
}

pub fn get<H: HostApi>(host: &mut H, server_id: u64, username: &str) -> Result<FtpUser, ApiError> {
    store::get_user(host, server_id, username)?.ok_or_else(user_not_found)
}

pub fn list<H: HostApi>(host: &mut H, server_id: u64) -> Result<Vec<FtpUser>, ApiError> {
    store::list_users_by_server(host, server_id)
}

pub fn update<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
    input: UpdateUserInput,
) -> Result<FtpUser, ApiError> {
    let mut user = get(host, server_id, username)?;

    if let Some(password) = input.password.as_deref()
        && !password.is_empty()
    {
        user.password_hash = password::hash_password(host, password)?;
    }
    if let Some(home_dir) = input.home_dir {
        user.home_dir = home_dir;
    }
    if let Some(quota_bytes) = input.quota_bytes {
        user.quota_bytes = quota_bytes;
    }
    if let Some(enabled) = input.enabled {
        user.enabled = enabled;
    }
    if let Some(description) = input.description {
        user.description = description;
    }

    store::save_user(host, server_id, &user)?;
    Ok(user)
}

pub fn delete<H: HostApi>(host: &mut H, server_id: u64, username: &str) -> Result<(), ApiError> {
    get(host, server_id, username)?;
    store::delete_user(host, server_id, username)?;
    if let Err(err) = store::remove_from_user_list(host, server_id, username) {
        host.log_warn(&format!("failed to update user list: {}", err.message));
    }
    Ok(())
}

pub fn update_access_rules<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
    rules: Vec<AccessRule>,
) -> Result<FtpUser, ApiError> {
    let mut user = get(host, server_id, username)?;
    user.access_rules = rules;
    store::save_user(host, server_id, &user)?;
    Ok(user)
}

pub fn update_virtual_paths<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
    paths: Vec<VirtualPath>,
) -> Result<FtpUser, ApiError> {
    let mut user = get(host, server_id, username)?;
    user.virtual_paths = paths;
    store::save_user(host, server_id, &user)?;
    Ok(user)
}

pub fn add_ssh_key<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
    key: String,
) -> Result<FtpUser, ApiError> {
    let mut user = get(host, server_id, username)?;
    if !domain::is_valid_ssh_key(&key) {
        return Err(ApiError::bad_request(ERR_INVALID_SSH_KEY));
    }
    user.ssh_public_keys.push(key);
    store::save_user(host, server_id, &user)?;
    Ok(user)
}

pub fn remove_ssh_key<H: HostApi>(
    host: &mut H,
    server_id: u64,
    username: &str,
    index: i64,
) -> Result<FtpUser, ApiError> {
    let mut user = get(host, server_id, username)?;
    // Go returned a generic error here, surfacing as a 500 — kept for parity.
    if index < 0 || index as usize >= user.ssh_public_keys.len() {
        return Err(ApiError::internal("invalid key index"));
    }
    user.ssh_public_keys.remove(index as usize);
    store::save_user(host, server_id, &user)?;
    Ok(user)
}

/// Deletes every user of a server, returning the usernames that were listed
/// (used by the SERVER_DELETED event to clean node files afterwards). The
/// user-list key is reset to `[]`, never deleted — Go behavior.
pub fn delete_all_by_server<H: HostApi>(
    host: &mut H,
    server_id: u64,
) -> Result<Vec<String>, ApiError> {
    let usernames = store::get_user_list(host, server_id)?;
    for username in &usernames {
        if let Err(err) = store::delete_user(host, server_id, username) {
            host.log_warn(&format!("failed to delete user {username}: {}", err.message));
        }
    }
    if let Err(err) = store::update_user_list(host, server_id, &[]) {
        host.log_warn(&format!("failed to clear user list: {}", err.message));
    }
    Ok(usernames)
}
