//! Per-route handlers: parse → service → JSON response. Thin by design —
//! behavior (status codes, error messages, sync-is-best-effort) mirrors the
//! Go handlers.

pub mod admin;
pub mod events;
pub mod nodes;
pub mod users;

#[cfg(test)]
mod tests;

use crate::host_api::HostApi;
use crate::http::ApiResult;
use crate::router::{RequestParts, RouteId};

pub fn handle<H: HostApi>(host: &mut H, route: RouteId, parts: &RequestParts) -> ApiResult {
    match route {
        RouteId::AdminNodes => admin::list_nodes(host),
        RouteId::AdminUsers => admin::list_users(host, parts),
        RouteId::AdminPickerServers => admin::picker_servers(host, parts),
        RouteId::NodeSetup => nodes::setup(host, parts),
        RouteId::NodeStatus => nodes::status(host, parts),
        RouteId::NodeConfigGet => nodes::get_config(host, parts),
        RouteId::NodeConfigUpdate => nodes::update_config(host, parts),
        RouteId::UsersList => users::list(host, parts),
        RouteId::UserCreate => users::create(host, parts),
        RouteId::UserGet => users::get(host, parts),
        RouteId::UserUpdate => users::update(host, parts),
        RouteId::UserDelete => users::delete(host, parts),
        RouteId::AccessRulesGet => users::get_access_rules(host, parts),
        RouteId::AccessRulesUpdate => users::update_access_rules(host, parts),
        RouteId::VirtualPathsGet => users::get_virtual_paths(host, parts),
        RouteId::VirtualPathsUpdate => users::update_virtual_paths(host, parts),
        RouteId::SshKeysList => users::list_ssh_keys(host, parts),
        RouteId::SshKeyAdd => users::add_ssh_key(host, parts),
        RouteId::SshKeyDelete => users::delete_ssh_key(host, parts),
    }
}
