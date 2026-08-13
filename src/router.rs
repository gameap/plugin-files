//! Route table and dispatch. The matcher mirrors the host's `matchPath`
//! (segment-wise comparison, `{name}` captures, first match wins), so the
//! declared table stays the single source of truth. Patterns, methods, auth
//! flags and descriptions are copied verbatim from the Go plugin's
//! `internal/handlers/router.go` — this table is the public API contract.
//! `/admin/pickers/servers` is a Rust-era additive route with no Go
//! counterpart.

use std::collections::HashMap;

use gameap_plugin_sdk::proto::gameap::plugin as pb;

use crate::handlers;
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RouteId {
    AdminNodes,
    AdminUsers,
    AdminPickerServers,
    NodeSetup,
    NodeStatus,
    NodeConfigGet,
    NodeConfigUpdate,
    UsersList,
    UserCreate,
    UserGet,
    UserUpdate,
    UserDelete,
    AccessRulesGet,
    AccessRulesUpdate,
    VirtualPathsGet,
    VirtualPathsUpdate,
    SshKeysList,
    SshKeyAdd,
    SshKeyDelete,
}

pub struct RouteDef {
    pub id: RouteId,
    pub method: &'static str,
    pub pattern: &'static str,
    pub admin_only: bool,
    pub description: &'static str,
}

pub const ROUTES: &[RouteDef] = &[
    RouteDef {
        id: RouteId::AdminNodes,
        method: "GET",
        pattern: "/admin/nodes",
        admin_only: true,
        description: "List all nodes with plugin status",
    },
    RouteDef {
        id: RouteId::AdminUsers,
        method: "GET",
        pattern: "/admin/users",
        admin_only: true,
        description: "List all FTP users across all servers",
    },
    RouteDef {
        id: RouteId::AdminPickerServers,
        method: "GET",
        pattern: "/admin/pickers/servers",
        admin_only: true,
        description: "Search game servers for the create-user picker",
    },
    RouteDef {
        id: RouteId::NodeSetup,
        method: "POST",
        pattern: "/nodes/{nodeId}/setup",
        admin_only: true,
        description: "Setup gameap-files on a node",
    },
    RouteDef {
        id: RouteId::NodeStatus,
        method: "GET",
        pattern: "/nodes/{nodeId}/status",
        admin_only: true,
        description: "Get gameap-files installation status",
    },
    RouteDef {
        id: RouteId::NodeConfigGet,
        method: "GET",
        pattern: "/nodes/{nodeId}/config",
        admin_only: true,
        description: "Get node FTP/SFTP configuration",
    },
    RouteDef {
        id: RouteId::NodeConfigUpdate,
        method: "PUT",
        pattern: "/nodes/{nodeId}/config",
        admin_only: true,
        description: "Update node FTP/SFTP configuration",
    },
    RouteDef {
        id: RouteId::UsersList,
        method: "GET",
        pattern: "/servers/{serverId}/ftp-users",
        admin_only: false,
        description: "List FTP users for a server",
    },
    RouteDef {
        id: RouteId::UserCreate,
        method: "POST",
        pattern: "/servers/{serverId}/ftp-users",
        admin_only: false,
        description: "Create FTP user",
    },
    RouteDef {
        id: RouteId::UserGet,
        method: "GET",
        pattern: "/servers/{serverId}/ftp-users/{username}",
        admin_only: false,
        description: "Get FTP user",
    },
    RouteDef {
        id: RouteId::UserUpdate,
        method: "PUT",
        pattern: "/servers/{serverId}/ftp-users/{username}",
        admin_only: false,
        description: "Update FTP user",
    },
    RouteDef {
        id: RouteId::UserDelete,
        method: "DELETE",
        pattern: "/servers/{serverId}/ftp-users/{username}",
        admin_only: false,
        description: "Delete FTP user",
    },
    RouteDef {
        id: RouteId::AccessRulesGet,
        method: "GET",
        pattern: "/servers/{serverId}/ftp-users/{username}/access-rules",
        admin_only: false,
        description: "Get access rules",
    },
    RouteDef {
        id: RouteId::AccessRulesUpdate,
        method: "PUT",
        pattern: "/servers/{serverId}/ftp-users/{username}/access-rules",
        admin_only: false,
        description: "Update access rules",
    },
    RouteDef {
        id: RouteId::VirtualPathsGet,
        method: "GET",
        pattern: "/servers/{serverId}/ftp-users/{username}/virtual-paths",
        admin_only: false,
        description: "Get virtual paths",
    },
    RouteDef {
        id: RouteId::VirtualPathsUpdate,
        method: "PUT",
        pattern: "/servers/{serverId}/ftp-users/{username}/virtual-paths",
        admin_only: false,
        description: "Update virtual paths",
    },
    RouteDef {
        id: RouteId::SshKeysList,
        method: "GET",
        pattern: "/servers/{serverId}/ftp-users/{username}/ssh-keys",
        admin_only: false,
        description: "List SSH keys",
    },
    RouteDef {
        id: RouteId::SshKeyAdd,
        method: "POST",
        pattern: "/servers/{serverId}/ftp-users/{username}/ssh-keys",
        admin_only: false,
        description: "Add SSH key",
    },
    RouteDef {
        id: RouteId::SshKeyDelete,
        method: "DELETE",
        pattern: "/servers/{serverId}/ftp-users/{username}/ssh-keys/{index}",
        admin_only: false,
        description: "Delete SSH key",
    },
];

pub fn http_routes() -> Vec<pb::HttpRoute> {
    ROUTES
        .iter()
        .map(|route| pb::HttpRoute {
            path: route.pattern.into(),
            methods: vec![route.method.into()],
            requires_auth: true,
            admin_only: route.admin_only,
            description: route.description.into(),
        })
        .collect()
}

pub fn match_route(method: &str, path: &str) -> Option<(RouteId, HashMap<String, String>)> {
    ROUTES.iter().find_map(|route| {
        if !route.method.eq_ignore_ascii_case(method) {
            return None;
        }
        match_pattern(route.pattern, path).map(|params| (route.id, params))
    })
}

fn match_pattern(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pattern_segments: Vec<&str> = pattern.trim_matches('/').split('/').collect();
    let path_segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    if pattern_segments.len() != path_segments.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (pat, seg) in pattern_segments.iter().zip(path_segments.iter()) {
        if let Some(name) = pat.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
            params.insert(name.to_string(), (*seg).to_string());
        } else if pat != seg {
            return None;
        }
    }
    Some(params)
}

/// Everything a handler needs from the incoming request. The Go plugin never
/// reads the session, so it is not carried here.
pub struct RequestParts<'a> {
    pub params: HashMap<String, String>,
    pub body: &'a [u8],
    pub query: &'a HashMap<String, pb::QueryParamValues>,
}

pub fn dispatch<H: HostApi>(host: &mut H, req: &pb::HttpRequest) -> pb::HttpResponse {
    let Some((route, params)) = match_route(&req.method, &req.path) else {
        return ApiError::not_found("route not found").into_response();
    };
    let parts = RequestParts {
        params,
        body: &req.body,
        query: &req.query_params,
    };
    let result: ApiResult = handlers::handle(host, route, &parts);
    result.unwrap_or_else(ApiError::into_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_every_declared_route() {
        let cases: &[(&str, &str, RouteId)] = &[
            ("GET", "/admin/nodes", RouteId::AdminNodes),
            ("GET", "/admin/users", RouteId::AdminUsers),
            ("GET", "/admin/pickers/servers", RouteId::AdminPickerServers),
            ("POST", "/nodes/1/setup", RouteId::NodeSetup),
            ("GET", "/nodes/1/status", RouteId::NodeStatus),
            ("GET", "/nodes/1/config", RouteId::NodeConfigGet),
            ("PUT", "/nodes/1/config", RouteId::NodeConfigUpdate),
            ("GET", "/servers/3/ftp-users", RouteId::UsersList),
            ("POST", "/servers/3/ftp-users", RouteId::UserCreate),
            ("GET", "/servers/3/ftp-users/bob", RouteId::UserGet),
            ("PUT", "/servers/3/ftp-users/bob", RouteId::UserUpdate),
            ("DELETE", "/servers/3/ftp-users/bob", RouteId::UserDelete),
            (
                "GET",
                "/servers/3/ftp-users/bob/access-rules",
                RouteId::AccessRulesGet,
            ),
            (
                "PUT",
                "/servers/3/ftp-users/bob/access-rules",
                RouteId::AccessRulesUpdate,
            ),
            (
                "GET",
                "/servers/3/ftp-users/bob/virtual-paths",
                RouteId::VirtualPathsGet,
            ),
            (
                "PUT",
                "/servers/3/ftp-users/bob/virtual-paths",
                RouteId::VirtualPathsUpdate,
            ),
            ("GET", "/servers/3/ftp-users/bob/ssh-keys", RouteId::SshKeysList),
            ("POST", "/servers/3/ftp-users/bob/ssh-keys", RouteId::SshKeyAdd),
            (
                "DELETE",
                "/servers/3/ftp-users/bob/ssh-keys/0",
                RouteId::SshKeyDelete,
            ),
        ];
        for (method, path, want) in cases {
            let (got, _) = match_route(method, path)
                .unwrap_or_else(|| panic!("{method} {path} must match"));
            assert_eq!(got, *want, "{method} {path}");
        }
    }

    #[test]
    fn captures_params() {
        let (_, params) = match_route("DELETE", "/servers/3/ftp-users/bob/ssh-keys/2").unwrap();
        assert_eq!(params.get("serverId").map(String::as_str), Some("3"));
        assert_eq!(params.get("username").map(String::as_str), Some("bob"));
        assert_eq!(params.get("index").map(String::as_str), Some("2"));
    }

    #[test]
    fn rejects_unknown() {
        assert!(match_route("GET", "/unknown").is_none());
        assert!(match_route("PATCH", "/servers/3/ftp-users").is_none());
        assert!(match_route("GET", "/servers/3/ftp-users/bob/extra/more").is_none());
        assert!(match_route("GET", "/").is_none());
    }

    #[test]
    fn auth_flags_match_go() {
        for route in http_routes() {
            assert!(route.requires_auth, "route {} must require auth", route.path);
            let is_admin_scope =
                route.path.starts_with("/admin/") || route.path.starts_with("/nodes/");
            assert_eq!(
                route.admin_only, is_admin_scope,
                "route {} admin_only flag",
                route.path
            );
        }
        assert_eq!(http_routes().len(), 19);
    }
}
