//! FTP user model. One struct serves both storage JSON and the node-side YAML
//! drop-in — exactly like the Go `domain.FTPUser`, whose json/yaml tags were
//! identical and had to match `gameap-files/internal/user/user.go`.

use serde::{Deserialize, Serialize};

use super::null_to_default;

pub const PERMISSION_READ: &str = "read";
pub const PERMISSION_WRITE: &str = "write";
pub const PERMISSION_DELETE: &str = "delete";
pub const PERMISSION_LIST: &str = "list";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FtpUser {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password_hash: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub ssh_public_keys: Vec<String>,
    #[serde(default)]
    pub home_dir: String,
    #[serde(default)]
    pub quota_bytes: i64,
    #[serde(default, deserialize_with = "null_to_default")]
    pub access_rules: Vec<AccessRule>,
    #[serde(default, deserialize_with = "null_to_default")]
    pub virtual_paths: Vec<VirtualPath>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessRule {
    #[serde(default)]
    pub path: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualPath {
    #[serde(default, rename = "virtual")]
    pub virtual_path: String,
    #[serde(default)]
    pub target: String,
    #[serde(default, deserialize_with = "null_to_default")]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub read_only: bool,
}

/// Full access to the whole tree — assigned to newly created users.
pub fn default_access_rules() -> Vec<AccessRule> {
    vec![AccessRule {
        path: "/**".into(),
        permissions: vec![
            PERMISSION_READ.into(),
            PERMISSION_WRITE.into(),
            PERMISSION_DELETE.into(),
            PERMISSION_LIST.into(),
        ],
    }]
}

/// Go regex `^[a-zA-Z][a-zA-Z0-9_]{2,31}$`, hand-rolled to avoid a regex
/// dependency in the wasm binary.
pub fn validate_username(username: &str) -> bool {
    let bytes = username.as_bytes();
    if !(3..=32).contains(&bytes.len()) || !username.is_ascii() {
        return false;
    }
    if !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// Go `isValidSSHKey`: prefix check only, and the key must be strictly longer
/// than the prefix (a bare "ssh-rsa " is invalid).
pub fn is_valid_ssh_key(key: &str) -> bool {
    const PREFIXES: [&str; 4] = ["ssh-rsa ", "ssh-ed25519 ", "ecdsa-sha2-", "ssh-dss "];
    PREFIXES
        .iter()
        .any(|prefix| key.len() > prefix.len() && key.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_validation_table() {
        let valid = ["bob", "Bob_123", "a__", "x2345678901234567890123456789012"];
        for name in valid {
            assert!(validate_username(name), "{name} must be valid");
        }
        let invalid = [
            "",
            "ab",
            "1abc",
            "_abc",
            "bob-1",
            "bob.name",
            "юзер",
            "x23456789012345678901234567890123", // 33 chars
            "bob name",
        ];
        for name in invalid {
            assert!(!validate_username(name), "{name} must be invalid");
        }
    }

    #[test]
    fn ssh_key_validation_table() {
        assert!(is_valid_ssh_key("ssh-ed25519 AAAAC3Nza user@host"));
        assert!(is_valid_ssh_key("ssh-rsa AAAAB3Nza"));
        assert!(is_valid_ssh_key("ecdsa-sha2-nistp256 AAAA"));
        assert!(is_valid_ssh_key("ssh-dss AAAA"));
        assert!(!is_valid_ssh_key("ssh-rsa ")); // bare prefix
        assert!(!is_valid_ssh_key("ssh-ed25519"));
        assert!(!is_valid_ssh_key("garbage"));
        assert!(!is_valid_ssh_key(""));
    }

    /// Byte-equal round-trip of a document exactly as Go's encoding/json wrote
    /// it (field order = struct declaration order, no omitted keys).
    #[test]
    fn storage_json_round_trip_full() {
        let go_json = concat!(
            r#"{"username":"bob","password_hash":"$argon2id$v=19$m=65536,t=3,p=4$c2FsdA$aGFzaA","#,
            r#""ssh_public_keys":["ssh-ed25519 AAAA bob@pc"],"home_dir":"/srv/gameap/servers/cs","#,
            r#""quota_bytes":10737418240,"access_rules":[{"path":"/**","permissions":["read","write","delete","list"]}],"#,
            r#""virtual_paths":[{"virtual":"/shared","target":"/srv/shared","permissions":["read","list"],"read_only":true}],"#,
            r#""enabled":true,"description":"main user"}"#
        );
        let user: FtpUser = serde_json::from_str(go_json).unwrap();
        assert_eq!(user.username, "bob");
        assert_eq!(user.virtual_paths[0].virtual_path, "/shared");
        assert!(user.virtual_paths[0].read_only);
        assert_eq!(serde_json::to_string(&user).unwrap(), go_json);
    }

    /// Go wrote nil slices as null (e.g. after `PUT access-rules {"rules":null}`).
    #[test]
    fn storage_json_tolerates_null_slices() {
        let go_json = concat!(
            r#"{"username":"bob","password_hash":"h","ssh_public_keys":null,"home_dir":"","#,
            r#""quota_bytes":0,"access_rules":null,"virtual_paths":null,"enabled":false,"description":""}"#
        );
        let user: FtpUser = serde_json::from_str(go_json).unwrap();
        assert!(user.ssh_public_keys.is_empty());
        assert!(user.access_rules.is_empty());
        assert!(user.virtual_paths.is_empty());
    }

    /// The YAML drop-in must keep the exact key set/names the daemon expects.
    #[test]
    fn yaml_shape_for_daemon() {
        let user = FtpUser {
            username: "bob".into(),
            password_hash: "$argon2id$v=19$m=65536,t=3,p=4$c2FsdA$aGFzaA".into(),
            ssh_public_keys: vec![],
            home_dir: "/srv/gameap/servers/cs".into(),
            quota_bytes: 0,
            access_rules: default_access_rules(),
            virtual_paths: vec![VirtualPath {
                virtual_path: "/shared".into(),
                target: "/srv/shared".into(),
                permissions: vec!["read".into()],
                read_only: true,
            }],
            enabled: true,
            description: String::new(),
        };
        let yaml = serde_yaml_ng::to_string(&user).unwrap();
        for key in [
            "username:",
            "password_hash:",
            "ssh_public_keys:",
            "home_dir:",
            "quota_bytes:",
            "access_rules:",
            "virtual_paths:",
            "- virtual: /shared",
            "target: /srv/shared",
            "read_only: true",
            "enabled: true",
            "description:",
        ] {
            assert!(yaml.contains(key), "yaml must contain {key:?}:\n{yaml}");
        }
        // And it must parse back identically (daemon-side goccy/go-yaml is
        // strictly YAML 1.2 compatible for this subset).
        let back: FtpUser = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(back, user);
    }
}
