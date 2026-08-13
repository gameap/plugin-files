//! Cross-node/server aggregation for the admin pages
//! (Go `internal/service/admin.go`).

use std::collections::BTreeMap;

use crate::domain::{FtpUser, NodeSetupStatus, SetupStatus};
use crate::host_api::HostApi;
use crate::http::ApiError;
use crate::services::store;

pub struct AdminNode {
    pub id: u64,
    pub name: String,
    pub ip: String,
    pub status: NodeSetupStatus,
}

pub struct GroupedServer {
    pub server_id: u64,
    pub server_name: String,
    pub game_id: String,
    pub users: Vec<FtpUser>,
}

pub struct GroupedNode {
    pub node_id: u64,
    pub node_name: String,
    pub servers: BTreeMap<u64, GroupedServer>,
}

pub struct AdminUsersResult {
    pub grouped: BTreeMap<u64, GroupedNode>,
    pub total: i64,
}

#[derive(Default)]
pub struct UserFilters {
    pub search: String,
    pub node_id: u64,
    pub server_id: u64,
    pub enabled: Option<bool>,
}

pub fn list_all_nodes<H: HostApi>(host: &mut H) -> Result<Vec<AdminNode>, ApiError> {
    let nodes = host.find_nodes()?;
    let mut result = Vec::with_capacity(nodes.len());
    for node in nodes {
        let status = match store::get_status(host, node.id) {
            Ok(Some(status)) => status,
            Ok(None) => NodeSetupStatus::new(SetupStatus::NotInstalled),
            Err(err) => {
                host.log_warn(&format!(
                    "failed to get node status: node_id={} error={}",
                    node.id, err.message
                ));
                NodeSetupStatus::new(SetupStatus::NotInstalled)
            }
        };
        result.push(AdminNode {
            id: node.id,
            name: node.name,
            ip: node.ips.first().cloned().unwrap_or_default(),
            status,
        });
    }
    Ok(result)
}

pub fn list_all_users<H: HostApi>(
    host: &mut H,
    filters: &UserFilters,
) -> Result<AdminUsersResult, ApiError> {
    let servers = host.find_servers(&[], &[])?;
    let node_names: BTreeMap<u64, String> = host
        .find_nodes()?
        .into_iter()
        .map(|n| (n.id, n.name))
        .collect();

    let mut result = AdminUsersResult {
        grouped: BTreeMap::new(),
        total: 0,
    };

    for server in servers {
        if filters.node_id != 0 && server.node_id != filters.node_id {
            continue;
        }
        if filters.server_id != 0 && server.id != filters.server_id {
            continue;
        }

        let users = match store::list_users_by_server(host, server.id) {
            Ok(users) => users,
            Err(err) => {
                host.log_warn(&format!(
                    "failed to list users for server: server_id={} error={}",
                    server.id, err.message
                ));
                continue;
            }
        };
        if users.is_empty() {
            continue;
        }

        let search = filters.search.to_lowercase();
        let filtered: Vec<FtpUser> = users
            .into_iter()
            .filter(|user| {
                (search.is_empty() || user.username.to_lowercase().contains(&search))
                    && filters.enabled.is_none_or(|want| user.enabled == want)
            })
            .collect();
        if filtered.is_empty() {
            continue;
        }

        let node_id = server.node_id;
        let node = result.grouped.entry(node_id).or_insert_with(|| GroupedNode {
            node_id,
            node_name: match node_names.get(&node_id) {
                Some(name) if !name.is_empty() => name.clone(),
                _ => "Unknown Node".into(),
            },
            servers: BTreeMap::new(),
        });

        result.total += filtered.len() as i64;
        node.servers.insert(
            server.id,
            GroupedServer {
                server_id: server.id,
                server_name: server.name,
                game_id: server.game_id,
                users: filtered,
            },
        );
    }

    Ok(result)
}
