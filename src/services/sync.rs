//! Mirrors storage state to nodes: one YAML drop-in per user under
//! `<work_path>/.plugins/files/users.d/` (the daemon hot-reloads the
//! directory). Node paths are relative to the daemon work path on purpose:
//! gameap-daemon confines plugin file operations to that path, and
//! `.plugins/files` is the service directory the panel grants this plugin
//! under every path policy, on Linux and Windows nodes alike.
//! All sync failures are the caller's to downgrade — handlers log a warning
//! and keep the HTTP request successful, exactly like the Go plugin.

use crate::domain::FtpUser;
use crate::host_api::{HostApi, HostApiError};
use crate::http::ApiError;
use crate::services::store;

pub const USERS_CONFIG_DIR: &str = ".plugins/files/users.d";
/// Where uploads of releases before 0.8 landed: they named the absolute
/// `/etc/gameap-files/users.d`, which the daemon resolved under its work path
/// as `<work_path>/etc/gameap-files/users.d`. Never read by gameap-files;
/// swept after an installation finishes.
pub const LEGACY_MISPLACED_USERS_DIR: &str = "etc/gameap-files/users.d";
const USER_CONFIG_PERM: u32 = 0o600;

pub fn user_config_path(username: &str) -> String {
    format!("{USERS_CONFIG_DIR}/{username}.yaml")
}

pub fn legacy_misplaced_user_config_path(username: &str) -> String {
    format!("{LEGACY_MISPLACED_USERS_DIR}/{username}.yaml")
}

/// Node id of a server, with the Go plugin's "server %d not found" message
/// (surfaces as a 500, which the create/update handlers rely on).
pub fn node_id_for_server<H: HostApi>(host: &mut H, server_id: u64) -> Result<u64, ApiError> {
    let server = host
        .get_server(server_id)?
        .ok_or_else(|| ApiError::internal(format!("server {server_id} not found")))?;
    Ok(server.node_id)
}

/// Work path of a node (the daemon's data directory), a 500 when the node is
/// gone.
pub fn node_work_path<H: HostApi>(host: &mut H, node_id: u64) -> Result<String, ApiError> {
    let node = host
        .get_node(node_id)?
        .ok_or_else(|| ApiError::internal(format!("node {node_id} not found")))?;
    Ok(node.work_path)
}

/// Absolute on either OS: `/…`, `\…`, `X:\…`, `X:/…`.
pub fn is_absolute_node_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    if matches!(bytes.first(), Some(b'/' | b'\\')) {
        return true;
    }
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

/// Joins a work-path-relative entry onto the work path with the separator the
/// work path itself uses, so a Windows node gets a Windows path. An absolute
/// `rel` is returned as is.
pub fn join_node_path(work_path: &str, rel: &str) -> String {
    if is_absolute_node_path(rel) {
        return rel.to_string();
    }
    let separator = if work_path.contains('\\') { '\\' } else { '/' };
    let base = work_path.trim_end_matches(['/', '\\']);
    let rel = rel.trim_start_matches(['/', '\\']);
    if rel.is_empty() {
        return if base.is_empty() {
            work_path.to_string()
        } else {
            base.to_string()
        };
    }
    format!("{base}{separator}{rel}")
}

/// A home directory as the client asked for it: absolute paths are kept,
/// relative ones are anchored under the node work path.
pub fn resolve_home_dir(work_path: &str, requested: &str) -> String {
    join_node_path(work_path, requested)
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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ResyncReport {
    pub synced: usize,
    pub failed: usize,
    pub legacy_removed: usize,
}

/// Pushes every user of every server on the node to the users directory and
/// sweeps the misplaced files earlier releases left behind. Run once after an
/// installation finishes: an upgrade moves the users directory, and a node
/// installed by an older release has its users where nothing reads them.
/// Never fails — a user that cannot be synced is warned about and counted.
pub fn resync_node_users<H: HostApi>(host: &mut H, node_id: u64) -> ResyncReport {
    let mut report = ResyncReport::default();

    let servers = match host.find_servers(&[], &[node_id]) {
        Ok(servers) => servers,
        Err(err) => {
            host.log_warn(&format!(
                "resync: failed to list servers of node {node_id}: {}",
                err.message()
            ));
            return report;
        }
    };

    for server in servers {
        let users = match store::list_users_by_server(host, server.id) {
            Ok(users) => users,
            Err(err) => {
                host.log_warn(&format!(
                    "resync: failed to list users of server {}: {}",
                    server.id, err.message
                ));
                continue;
            }
        };
        for user in users {
            match upload_user_config(host, node_id, &user) {
                Ok(()) => report.synced += 1,
                Err(err) => {
                    report.failed += 1;
                    host.log_warn(&format!("resync: {}", err.message));
                }
            }
            // The daemon refuses to remove what is not there, which is the
            // normal case; only an actual removal counts.
            if host
                .remove(node_id, &legacy_misplaced_user_config_path(&user.username), false)
                .is_ok()
            {
                report.legacy_removed += 1;
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::default_access_rules;
    use crate::host_api::mock::MockHost;

    #[test]
    fn user_config_paths_are_work_path_relative() {
        assert_eq!(user_config_path("bob"), ".plugins/files/users.d/bob.yaml");
        assert_eq!(
            legacy_misplaced_user_config_path("bob"),
            "etc/gameap-files/users.d/bob.yaml"
        );
    }

    #[test]
    fn join_node_path_table() {
        let cases: &[(&str, &str, &str)] = &[
            ("/srv/gameap", "servers/cs", "/srv/gameap/servers/cs"),
            ("/srv/gameap/", "servers/cs", "/srv/gameap/servers/cs"),
            (r"C:\gameap", r"servers\cs2", r"C:\gameap\servers\cs2"),
            (r"C:\gameap\", "servers/cs2", r"C:\gameap\servers/cs2"),
            ("C:/gameap", "servers/cs", "C:/gameap/servers/cs"),
            ("/srv/gameap", "/x", "/x"),
            ("/srv/gameap", r"D:\x", r"D:\x"),
            (r"C:\gameap", "D:/x", "D:/x"),
            ("/srv/gameap", "", "/srv/gameap"),
            ("/srv/gameap/", "", "/srv/gameap"),
            ("/", "servers/cs", "/servers/cs"),
        ];
        for (work_path, rel, want) in cases {
            assert_eq!(join_node_path(work_path, rel), *want, "{work_path} + {rel}");
        }
    }

    #[test]
    fn absolute_node_paths() {
        assert!(is_absolute_node_path("/srv"));
        assert!(is_absolute_node_path(r"\srv"));
        assert!(is_absolute_node_path(r"C:\gameap"));
        assert!(is_absolute_node_path("c:/gameap"));
        assert!(!is_absolute_node_path("servers/cs"));
        assert!(!is_absolute_node_path("C:gameap"));
        assert!(!is_absolute_node_path(""));
    }

    #[test]
    fn resolve_home_dir_keeps_absolute_and_anchors_relative() {
        assert_eq!(resolve_home_dir("/srv/gameap", "/custom"), "/custom");
        assert_eq!(resolve_home_dir("/srv/gameap", "servers/a"), "/srv/gameap/servers/a");
        assert_eq!(resolve_home_dir(r"C:\gameap", "servers/a"), r"C:\gameap\servers/a");
    }

    fn user(name: &str) -> FtpUser {
        FtpUser {
            username: name.into(),
            password_hash: "h".into(),
            ssh_public_keys: vec![],
            home_dir: "/srv/gameap/servers/cs".into(),
            quota_bytes: 0,
            access_rules: default_access_rules(),
            virtual_paths: vec![],
            enabled: true,
            description: String::new(),
        }
    }

    #[test]
    fn resync_uploads_every_user_and_sweeps_legacy_files() {
        let mut host = MockHost::standard();
        store::save_user(&mut host, 3, &user("bob")).unwrap();
        store::save_user(&mut host, 3, &user("alice")).unwrap();

        let report = resync_node_users(&mut host, 1);
        assert_eq!(
            report,
            ResyncReport {
                synced: 2,
                failed: 0,
                legacy_removed: 2
            }
        );

        let mut uploaded: Vec<(u64, String, u32)> = host.uploads.clone();
        uploaded.sort();
        assert_eq!(
            uploaded,
            vec![
                (1, ".plugins/files/users.d/alice.yaml".to_string(), 0o600),
                (1, ".plugins/files/users.d/bob.yaml".to_string(), 0o600),
            ]
        );
        let mut removed: Vec<(u64, String, bool)> = host.removed.clone();
        removed.sort();
        assert_eq!(
            removed,
            vec![
                (1, "etc/gameap-files/users.d/alice.yaml".to_string(), false),
                (1, "etc/gameap-files/users.d/bob.yaml".to_string(), false),
            ]
        );
    }

    #[test]
    fn resync_of_a_node_without_servers_is_a_noop() {
        let mut host = MockHost::standard();
        let report = resync_node_users(&mut host, 42);
        assert_eq!(report, ResyncReport::default());
        assert!(host.uploads.is_empty());
        assert!(host.removed.is_empty());
    }
}
