//! GameAP plugin: manage the gameap-files FTP/SFTP daemon on nodes.
//!
//! The panel's KV storage is the source of truth for FTP users, access rules,
//! virtual paths and SSH keys; changes are mirrored to nodes as YAML drop-ins
//! under `/etc/gameap-files/users.d/` (the daemon hot-reloads them). Node
//! installation runs through chained daemon tasks; per-node FTP/SFTP settings
//! are patched into `/etc/gameap-files/config.yaml` in place, preserving keys
//! this plugin does not own.
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
            ..Default::default()
        })
    }

    fn initialize(
        &mut self,
        _req: pb::InitializeRequest,
    ) -> Result<pb::InitializeResponse, PluginError> {
        // No node calls at init — the host disables a plugin that overruns its
        // load deadline; all node work is request/event driven.
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
