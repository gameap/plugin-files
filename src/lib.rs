//! GameAP plugin: manage the gameap-files FTP/SFTP daemon on nodes.
//!
//! The panel's KV storage is the source of truth for FTP users, access rules,
//! virtual paths and SSH keys; changes are mirrored to nodes as YAML drop-ins
//! under `<work_path>/.plugins/files/users.d/` (the daemon hot-reloads them).
//! Every node path is relative to the daemon work path: gameap-daemon confines
//! plugin file operations to it, and `.plugins/files` is the service directory
//! the panel grants this plugin under every path policy. Node installation
//! runs through chained daemon tasks (Linux and Windows installers); per-node
//! FTP/SFTP settings are patched into `.plugins/files/config.yaml` in place,
//! preserving keys this plugin does not own.
//!
//! Rust port of the Go plugin `plugin-gameap-files`. Storage JSON, node YAML
//! and the HTTP API are wire-compatible with data written by the Go version.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod domain;
pub mod handlers;
pub mod host_api;
pub mod http;
pub mod router;
pub mod services;
pub mod shell;

use gameap_plugin_sdk::proto::gameap::plugin as pb;
use gameap_plugin_sdk::{Plugin, PluginError, register_plugin};

use crate::host_api::HostApi;

/// "files" is round-trip stable under the panel's `CompactPluginID` base32
/// normalization, so `/api/plugins/files/...` resolves literally. It is also
/// the id existing installs key their storage on — never change it.
pub const PLUGIN_ID: &str = "files";

/// What the plugin declares in its manifest and what it checks it was actually
/// given. One list, because two would drift.
///
/// - `files`: the users.d drop-ins and config.yaml are written and removed
///   through nodefs, and writes are the full grant; `files_read` covers only
///   the config.yaml download and is included in this one anyway.
/// - `listen_events`: without it `get_subscribed_events` is ignored and the
///   plugin never learns that a server was deleted or an install task finished.
/// - `manage_servers`: every daemon task the node installer chains is created
///   through it.
/// - `node_commands`: the version probe and the service restart are node
///   commands, and the installer's tasks are CMD_EXEC, which the panel gates on
///   this grant a second time.
pub const REQUIRED_PERMISSIONS: [&str; 4] = [
    "files",
    "listen_events",
    "manage_servers",
    "node_commands",
];

/// Says once, at load, which declared permissions the operator did not grant.
///
/// It is a diagnosis, not a gate. Whether a missing grant actually denies
/// anything depends on the panel's `PLUGINS_PERMISSIONS_ENFORCE`, which the host
/// does not expose — so refusing to work on the strength of this would break a
/// plugin that is running perfectly well, and the call that really is denied
/// already carries the panel's own "plugin permission ... required" message.
/// A host that cannot answer at all is not evidence of anything and is passed
/// over in silence.
fn report_missing_grants<H: HostApi>(host: &mut H) {
    let Ok(granted) = host.plugin_grants() else {
        return;
    };
    let missing: Vec<&str> = REQUIRED_PERMISSIONS
        .iter()
        .copied()
        .filter(|required| !granted.iter().any(|g| g == required))
        .collect();

    if !missing.is_empty() {
        host.log_error(&format!(
            "files: the panel has not granted {}; with permission enforcement on, \
             node installation and user synchronization will be refused until an \
             administrator adds them",
            missing.join(", "),
        ));
    }
}

const FRONTEND_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.js"));
const FRONTEND_CSS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.css"));

pub struct FilesPlugin<H> {
    host: H,
}

impl<H> FilesPlugin<H> {
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: HostApi> Plugin for FilesPlugin<H> {
    fn get_info(&mut self, _req: pb::GetInfoRequest) -> Result<pb::PluginInfo, PluginError> {
        Ok(pb::PluginInfo {
            id: PLUGIN_ID.into(),
            name: "FTP/SFTP Management".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "Manage FTP/SFTP users and access for game servers".into(),
            author: "GameAP".into(),
            api_version: "1".into(),
            required_permissions: REQUIRED_PERMISSIONS
                .iter()
                .map(|p| (*p).to_string())
                .collect(),
            ..Default::default()
        })
    }

    fn initialize(
        &mut self,
        _req: pb::InitializeRequest,
    ) -> Result<pb::InitializeResponse, PluginError> {
        // No node calls at init — the host disables a plugin that overruns its
        // load deadline; all node work is request/event driven. The grants
        // question is a host-side read only.
        report_missing_grants(&mut self.host);
        self.host.log_info("plugin initialized");
        Ok(pb::InitializeResponse {
            result: Some(gameap_plugin_sdk::ok_result()),
        })
    }

    fn shutdown(&mut self, _req: pb::ShutdownRequest) -> Result<pb::ShutdownResponse, PluginError> {
        self.host.log_info("plugin shutdown");
        Ok(pb::ShutdownResponse {
            result: Some(gameap_plugin_sdk::ok_result()),
        })
    }

    fn get_http_routes(
        &mut self,
        _req: pb::GetHttpRoutesRequest,
    ) -> Result<pb::GetHttpRoutesResponse, PluginError> {
        Ok(pb::GetHttpRoutesResponse {
            routes: router::http_routes(),
        })
    }

    fn handle_http_request(
        &mut self,
        req: pb::HttpRequest,
    ) -> Result<pb::HttpResponse, PluginError> {
        // Total dispatch: every failure becomes a JSON error response. An Err
        // here would surface as a plain-text host 500.
        Ok(router::dispatch(&mut self.host, &req))
    }

    fn get_subscribed_events(
        &mut self,
        _req: pb::GetSubscribedEventsRequest,
    ) -> Result<pb::GetSubscribedEventsResponse, PluginError> {
        Ok(pb::GetSubscribedEventsResponse {
            events: vec![
                pb::EventType::ServerDeleted as i32,
                pb::EventType::DaemonTaskCompleted as i32,
                pb::EventType::DaemonTaskFailed as i32,
            ],
        })
    }

    fn handle_event(&mut self, event: pb::Event) -> Result<pb::EventResult, PluginError> {
        Ok(handlers::events::handle(&mut self.host, &event))
    }

    fn get_server_abilities(
        &mut self,
        _req: pb::GetServerAbilitiesRequest,
    ) -> Result<pb::GetServerAbilitiesResponse, PluginError> {
        // Titles are translation keys resolved by the panel's privilege UI from
        // this plugin's frontend translations (plugins.<id>.<key>).
        Ok(pb::GetServerAbilitiesResponse {
            abilities: vec![
                pb::ServerAbility {
                    name: "ftp-users-manage".into(),
                    title: "plugins.files.abilities.ftp-users-manage".into(),
                },
                pb::ServerAbility {
                    name: "ftp-users-view".into(),
                    title: "plugins.files.abilities.ftp-users-view".into(),
                },
            ],
        })
    }

    fn get_frontend_bundle(
        &mut self,
        _req: pb::GetFrontendBundleRequest,
    ) -> Result<pb::GetFrontendBundleResponse, PluginError> {
        Ok(pb::GetFrontendBundleResponse {
            bundle: FRONTEND_JS.to_vec(),
            has_bundle: !FRONTEND_JS.is_empty(),
            styles: FRONTEND_CSS.to_vec(),
            has_styles: !FRONTEND_CSS.is_empty(),
        })
    }
}

register_plugin!(
    FilesPlugin<host_api::WasmHost>,
    FilesPlugin::new(host_api::WasmHost)
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_api::mock::MockHost;

    fn manifest(host: MockHost) -> pb::PluginInfo {
        FilesPlugin::new(host)
            .get_info(pb::GetInfoRequest {})
            .expect("manifest")
    }

    #[test]
    fn the_manifest_declares_every_permission_the_plugin_needs() {
        let info = manifest(MockHost::default());
        assert_eq!(info.required_permissions, REQUIRED_PERMISSIONS.to_vec());
    }

    #[test]
    fn files_and_node_commands_are_declared() {
        // The two that carry the whole plugin: every user write reaches the node
        // through nodefs, and every install step is a node command. A manifest
        // without them installs and then fails on the first node round-trip.
        let declared = manifest(MockHost::default()).required_permissions;
        assert!(declared.iter().any(|p| p == "files"));
        assert!(declared.iter().any(|p| p == "node_commands"));
    }

    #[test]
    fn files_read_is_left_out_because_files_covers_it() {
        // The panel derives used permissions from the module's imports and drops
        // files_read when files is present; declaring both would make the
        // manifest disagree with the upload dry-run report.
        let declared = manifest(MockHost::default()).required_permissions;
        assert!(!declared.iter().any(|p| p == "files_read"));
    }

    #[test]
    fn a_missing_grant_is_named_once_at_load() {
        let mut host = MockHost::default();
        host.plugin_grants = vec![
            "files".into(),
            "listen_events".into(),
            "manage_servers".into(),
        ];

        report_missing_grants(&mut host);

        let logged = host.logs.join("\n");
        assert!(logged.contains("node_commands"), "{logged}");
        assert!(logged.starts_with("ERROR"), "{logged}");
        assert_eq!(host.logs.len(), 1, "{:?}", host.logs);
    }

    #[test]
    fn a_complete_grant_set_says_nothing() {
        let mut host = MockHost::default();
        host.plugin_grants = REQUIRED_PERMISSIONS
            .iter()
            .map(|p| (*p).to_string())
            .collect();

        report_missing_grants(&mut host);

        assert!(host.logs.is_empty(), "{:?}", host.logs);
    }

    #[test]
    fn a_host_that_cannot_answer_is_not_evidence_of_a_missing_grant() {
        // Reporting every permission as missing because the question failed
        // would be a lie, and an alarming one.
        let mut host = MockHost::default();
        host.plugin_grants_error = Some("unknown import".into());

        report_missing_grants(&mut host);

        assert!(host.logs.is_empty(), "{:?}", host.logs);
    }
}
