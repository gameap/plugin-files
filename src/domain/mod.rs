//! Typed domain model, wire-compatible with the Go plugin's structs.
//!
//! Field order in every struct matches the Go declaration order so that
//! serde_json's output is byte-identical to `encoding/json`'s (the round-trip
//! fixtures in the module tests pin this). Go marshals nil slices as `null`,
//! so every `Vec` field deserializes through [`null_to_default`].

pub mod node;
pub mod user;

pub use node::{
    FtpConfig, NodeConfig, NodeOs, NodeSetupStatus, SetupStatus, SftpConfig,
    default_node_config, extract_semver,
};
pub use user::{AccessRule, FtpUser, VirtualPath, default_access_rules, is_valid_ssh_key,
    validate_username};

/// Accepts JSON/YAML `null` where Go wrote a nil slice/map, yielding the
/// default value instead of a deserialization error.
pub fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + serde::Deserialize<'de>,
{
    use serde::Deserialize;
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}
