//! Admin endpoints (Go `internal/handlers/admin.go`). Grouped maps use
//! BTreeMap so JSON object keys are emitted deterministically — Go's map
//! order was random, so any order is compatible; keys serialize as strings
//! either way.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::host_api::HostApi;
use crate::http::{ApiResult, json_response, query_param};
use crate::router::RequestParts;
use crate::services::{admin, pickers};

use super::nodes::NodeStatusResponse;

#[derive(Serialize)]
struct AdminNodeResponse {
    id: u64,
    name: String,
    ip: String,
    plugin_status: NodeStatusResponse,
}

#[derive(Serialize)]
struct AdminNodesResponse {
    nodes: Vec<AdminNodeResponse>,
}

#[derive(Serialize)]
struct AdminUserResponse {
    username: String,
    enabled: bool,
    home_dir: String,
    quota_bytes: i64,
    description: String,
}

#[derive(Serialize)]
struct AdminGroupedServerResponse {
    server_id: u64,
    server_name: String,
    game_id: String,
    users: Vec<AdminUserResponse>,
}

#[derive(Serialize)]
struct AdminGroupedNodeResponse {
    node_id: u64,
    node_name: String,
    servers: BTreeMap<u64, AdminGroupedServerResponse>,
}

#[derive(Serialize)]
struct AdminUsersResponse {
    grouped: BTreeMap<u64, AdminGroupedNodeResponse>,
    total: i64,
}

pub fn list_nodes<H: HostApi>(host: &mut H) -> ApiResult {
    let nodes = admin::list_all_nodes(host)?;
    let response = AdminNodesResponse {
        nodes: nodes
            .into_iter()
            .map(|node| AdminNodeResponse {
                id: node.id,
                name: node.name,
                ip: node.ip,
                plugin_status: NodeStatusResponse::from(&node.status),
            })
            .collect(),
    };
    Ok(json_response(200, &response))
}

pub fn list_users<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let mut filters = admin::UserFilters {
        search: query_param(parts.query, "search").to_string(),
        ..Default::default()
    };
    if let Ok(node_id) = query_param(parts.query, "node_id").parse::<u64>() {
        filters.node_id = node_id;
    }
    if let Ok(server_id) = query_param(parts.query, "server_id").parse::<u64>() {
        filters.server_id = server_id;
    }
    let enabled_raw = query_param(parts.query, "enabled");
    if !enabled_raw.is_empty() {
        // Go quirk: any value other than "true" filters for disabled users.
        filters.enabled = Some(enabled_raw == "true");
    }

    let result = admin::list_all_users(host, &filters)?;

    let response = AdminUsersResponse {
        total: result.total,
        grouped: result
            .grouped
            .into_iter()
            .map(|(node_id, node)| {
                (
                    node_id,
                    AdminGroupedNodeResponse {
                        node_id: node.node_id,
                        node_name: node.node_name,
                        servers: node
                            .servers
                            .into_iter()
                            .map(|(server_id, server)| {
                                (
                                    server_id,
                                    AdminGroupedServerResponse {
                                        server_id: server.server_id,
                                        server_name: server.server_name,
                                        game_id: server.game_id,
                                        users: server
                                            .users
                                            .into_iter()
                                            .map(|user| AdminUserResponse {
                                                username: user.username,
                                                enabled: user.enabled,
                                                home_dir: user.home_dir,
                                                quota_bytes: user.quota_bytes,
                                                description: user.description,
                                            })
                                            .collect(),
                                    },
                                )
                            })
                            .collect(),
                    },
                )
            })
            .collect(),
    };
    Ok(json_response(200, &response))
}

pub fn picker_servers<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    pickers::servers(host, parts)
}
