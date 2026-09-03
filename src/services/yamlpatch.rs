//! In-place patching of the gameap-files daemon `config.yaml`.
//!
//! Only the seven keys this plugin owns are touched; everything else in the
//! file (server, security, logging, users, ftp.enabled, tls cert paths, …)
//! must survive the round-trip. Comments are lost — the Go version's
//! `map[string]interface{}` re-marshal lost them too.

use serde_yaml_ng::{Mapping, Value};

use crate::domain::NodeConfig;

pub fn patch_config(original: &[u8], config: &NodeConfig) -> Result<Vec<u8>, String> {
    let mut root: Value = if original.is_empty() {
        Value::Mapping(Mapping::new())
    } else {
        serde_yaml_ng::from_slice(original)
            .map_err(|err| format!("failed to parse config: {err}"))?
    };
    if root.is_null() {
        root = Value::Mapping(Mapping::new());
    }
    let Value::Mapping(doc) = &mut root else {
        return Err("failed to parse config: root is not a mapping".into());
    };

    if let Some(ftp) = &config.ftp {
        let ftp_map = ensure_map(doc, "ftp");
        insert(ftp_map, "listen_addr", format!("{}:{}", ftp.address, ftp.port));
        insert(ftp_map, "passive_port_min", ftp.passive_port_min);
        insert(ftp_map, "passive_port_max", ftp.passive_port_max);
        insert(ftp_map, "public_host", ftp.public_host.clone());

        let tls_map = ensure_map(ftp_map, "tls");
        insert(tls_map, "enabled", ftp.tls_enabled);
        // The daemon config keeps the implicit port as a listen suffix (":990").
        insert(tls_map, "implicit_port", format!(":{}", ftp.tls_implicit_port));
    }

    if let Some(sftp) = &config.sftp {
        let sftp_map = ensure_map(doc, "sftp");
        insert(sftp_map, "listen_addr", format!(":{}", sftp.port));
    }

    serde_yaml_ng::to_string(&root)
        .map(String::into_bytes)
        .map_err(|err| format!("failed to serialize config: {err}"))
}

fn insert(map: &mut Mapping, key: &str, value: impl Into<Value>) {
    map.insert(Value::String(key.to_string()), value.into());
}

/// Returns the nested mapping under `key`, replacing a missing or non-mapping
/// value with a fresh one (the Go version's failed type assertion did the same).
fn ensure_map<'a>(map: &'a mut Mapping, key: &str) -> &'a mut Mapping {
    let entry = map
        .entry(Value::String(key.to_string()))
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    if !entry.is_mapping() {
        *entry = Value::Mapping(Mapping::new());
    }
    match entry {
        Value::Mapping(m) => m,
        _ => unreachable!("entry was just set to a mapping"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::default_node_config;

    /// Shape of the config the install script writes (abridged from
    /// gameap-files' config.example.yaml) — unknown keys must survive.
    const DAEMON_CONFIG: &str = r#"server:
  data_dir: /srv/gameap
ftp:
  enabled: true
  listen_addr: ":21"
  passive_port_min: 30000
  passive_port_max: 30100
  public_host: ""
  idle_timeout: 300
  tls:
    enabled: false
    implicit_port: ":990"
    cert_file: tls/server.crt
sftp:
  enabled: true
  listen_addr: ":2222"
  host_key_file: ssh/host_ed25519_key
security:
  argon2:
    memory: 65536
    time: 3
    parallelism: 4
users:
  hot_reload: true
logging:
  level: info
"#;

    #[test]
    fn patches_owned_keys_and_preserves_the_rest() {
        let mut config = default_node_config();
        {
            let ftp = config.ftp.as_mut().unwrap();
            ftp.address = "0.0.0.0".into();
            ftp.port = 2121;
            ftp.passive_port_min = 40000;
            ftp.passive_port_max = 40100;
            ftp.public_host = "ftp.example.com".into();
            ftp.tls_enabled = true;
            ftp.tls_implicit_port = 991;
            config.sftp.as_mut().unwrap().port = 2223;
        }

        let patched = patch_config(DAEMON_CONFIG.as_bytes(), &config).unwrap();
        let value: Value = serde_yaml_ng::from_slice(&patched).unwrap();

        assert_eq!(value["ftp"]["listen_addr"], Value::from("0.0.0.0:2121"));
        assert_eq!(value["ftp"]["passive_port_min"], Value::from(40000));
        assert_eq!(value["ftp"]["passive_port_max"], Value::from(40100));
        assert_eq!(value["ftp"]["public_host"], Value::from("ftp.example.com"));
        assert_eq!(value["ftp"]["tls"]["enabled"], Value::from(true));
        assert_eq!(value["ftp"]["tls"]["implicit_port"], Value::from(":991"));
        assert_eq!(value["sftp"]["listen_addr"], Value::from(":2223"));

        // Untouched keys survive, with their types intact.
        assert_eq!(value["server"]["data_dir"], Value::from("/srv/gameap"));
        assert_eq!(value["ftp"]["enabled"], Value::from(true));
        assert_eq!(value["ftp"]["idle_timeout"], Value::from(300));
        assert_eq!(
            value["ftp"]["tls"]["cert_file"],
            Value::from("tls/server.crt")
        );
        assert_eq!(value["sftp"]["enabled"], Value::from(true));
        assert_eq!(value["security"]["argon2"]["memory"], Value::from(65536));
        assert_eq!(value["users"]["hot_reload"], Value::from(true));
        assert_eq!(value["logging"]["level"], Value::from("info"));
    }

    #[test]
    fn builds_sections_from_empty_input() {
        let patched = patch_config(b"", &default_node_config()).unwrap();
        let value: Value = serde_yaml_ng::from_slice(&patched).unwrap();
        assert_eq!(value["ftp"]["listen_addr"], Value::from(":21"));
        assert_eq!(value["ftp"]["tls"]["implicit_port"], Value::from(":990"));
        assert_eq!(value["sftp"]["listen_addr"], Value::from(":2222"));
    }

    #[test]
    fn section_skipped_when_absent_from_config() {
        let config = NodeConfig {
            ftp: None,
            sftp: config_sftp(2222),
        };
        let patched = patch_config(DAEMON_CONFIG.as_bytes(), &config).unwrap();
        let value: Value = serde_yaml_ng::from_slice(&patched).unwrap();
        // ftp untouched.
        assert_eq!(value["ftp"]["listen_addr"], Value::from(":21"));
        assert_eq!(value["sftp"]["listen_addr"], Value::from(":2222"));
    }

    fn config_sftp(port: i64) -> Option<crate::domain::SftpConfig> {
        Some(crate::domain::SftpConfig { port })
    }
}
