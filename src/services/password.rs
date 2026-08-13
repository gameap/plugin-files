//! Password generation and hashing through the host crypto service.

use crate::host_api::{Argon2Params, HostApi};
use crate::http::ApiError;

pub const PASSWORD_LENGTH: i32 = 16;
pub const PASSWORD_CHARSET: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Argon2id parameters matching the `security.argon2` block the install
/// script writes into the gameap-files daemon config — hashes must verify
/// there, so these are pinned.
pub const ARGON2_PARAMS: Argon2Params = Argon2Params {
    memory: 65536,
    time: 3,
    parallelism: 4,
    salt_length: 16,
    key_length: 32,
};

/// 16 chars of `[a-zA-Z0-9]` from the host CSPRNG. The charset is passed
/// explicitly: the host's default charset differs, and host-side generation
/// is unbiased (the Go plugin's local modulo-based generator was not).
pub fn generate_password<H: HostApi>(host: &mut H) -> Result<String, ApiError> {
    host.random_string(PASSWORD_LENGTH, Some(PASSWORD_CHARSET))
        .map_err(|err| ApiError::internal(format!("failed to generate password: {}", err.message())))
}

pub fn hash_password<H: HostApi>(host: &mut H, password: &str) -> Result<String, ApiError> {
    host.argon2_hash(password, ARGON2_PARAMS)
        .map_err(|err| ApiError::internal(format!("hash error: {}", err.message())))
}
