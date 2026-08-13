//! Panel server lookup for the admin create-user picker. Filtering and
//! sorting happen guest-side — the host's find filters are exact-match only
//! and its sorting field goes raw into ORDER BY.

use serde::Serialize;

use crate::host_api::{HostApi, ServerInfo};
use crate::http::{ApiResult, json_response, query_param};
use crate::router::RequestParts;

pub const DEFAULT_LIMIT: usize = 20;
pub const MAX_LIMIT: usize = 100;

#[derive(Serialize)]
struct PickerResponse {
    items: Vec<ServerItem>,
    /// Matched count before truncation (may exceed `items.len()`).
    total: usize,
}

#[derive(Serialize)]
struct ServerItem {
    id: u64,
    name: String,
    node_id: u64,
    enabled: bool,
    game_id: String,
}

fn parse_limit(parts: &RequestParts) -> usize {
    query_param(parts.query, "limit")
        .parse::<usize>()
        .ok()
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_LIMIT)
        .min(MAX_LIMIT)
}

fn server_matches(server: &ServerInfo, q: &str) -> bool {
    q.is_empty() || server.name.to_lowercase().contains(q)
}

/// GET /admin/pickers/servers?q=&node=&limit=
pub fn servers<H: HostApi>(host: &mut H, parts: &RequestParts) -> ApiResult {
    let q = query_param(parts.query, "q").to_lowercase();
    let limit = parse_limit(parts);
    let node_ids: Vec<u64> = query_param(parts.query, "node")
        .parse::<u64>()
        .ok()
        .into_iter()
        .collect();
    let mut servers = host.find_servers(&[], &node_ids)?;
    servers.retain(|s| server_matches(s, &q));
    servers.sort_by(|a, b| (a.name.to_lowercase(), a.id).cmp(&(b.name.to_lowercase(), b.id)));
    let total = servers.len();
    servers.truncate(limit);
    let items: Vec<ServerItem> = servers
        .into_iter()
        .map(|s| ServerItem {
            id: s.id,
            name: s.name,
            node_id: s.node_id,
            enabled: s.enabled,
            game_id: s.game_id,
        })
        .collect();
    Ok(json_response(200, &PickerResponse { items, total }))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use gameap_plugin_sdk::proto::gameap::plugin as pb;
    use serde_json::Value;

    use super::*;
    use crate::host_api::mock::MockHost;

    fn query(pairs: &[(&str, &str)]) -> HashMap<String, pb::QueryParamValues> {
        pairs
            .iter()
            .map(|(key, value)| {
                (
                    key.to_string(),
                    pb::QueryParamValues {
                        values: vec![value.to_string()],
                    },
                )
            })
            .collect()
    }

    fn parts(query: &HashMap<String, pb::QueryParamValues>) -> RequestParts<'_> {
        RequestParts {
            params: HashMap::new(),
            body: b"",
            query,
        }
    }

    fn server(id: u64, node_id: u64, name: &str) -> ServerInfo {
        ServerInfo {
            id,
            node_id,
            name: name.into(),
            game_id: "cstrike".into(),
            dir: format!("/srv/{name}"),
            enabled: true,
        }
    }

    #[test]
    fn match_is_case_insensitive_on_name() {
        let s = server(1, 1, "Public Deathmatch");
        assert!(server_matches(&s, ""));
        assert!(server_matches(&s, "deathmatch"));
        assert!(server_matches(&s, "public d"));
        assert!(!server_matches(&s, "cstrike"), "game_id is not searched");
    }

    #[test]
    fn filters_by_node_sorts_by_name_and_reports_total() {
        let mut host = MockHost::default();
        host.servers.insert(1, server(1, 1, "bravo"));
        host.servers.insert(2, server(2, 1, "Alpha"));
        host.servers.insert(3, server(3, 2, "other-node"));

        let query = query(&[("node", "1"), ("limit", "1")]);
        let resp = servers(&mut host, &parts(&query)).expect("picker response");
        assert_eq!(resp.status_code, 200);
        let body: Value = serde_json::from_slice(&resp.body).expect("json body");
        assert_eq!(body["total"], 2, "total counts matches before truncation");
        let items = body["items"].as_array().expect("items array");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "Alpha", "sorted by lowercased name");
        assert_eq!(items[0]["id"], 2);
        assert_eq!(items[0]["node_id"], 1);
    }

    #[test]
    fn limit_defaults_and_caps() {
        let empty = query(&[]);
        assert_eq!(parse_limit(&parts(&empty)), DEFAULT_LIMIT);
        let zero = query(&[("limit", "0")]);
        assert_eq!(parse_limit(&parts(&zero)), DEFAULT_LIMIT);
        let junk = query(&[("limit", "abc")]);
        assert_eq!(parse_limit(&parts(&junk)), DEFAULT_LIMIT);
        let above = query(&[("limit", "101")]);
        assert_eq!(parse_limit(&parts(&above)), MAX_LIMIT);
        let within = query(&[("limit", "5")]);
        assert_eq!(parse_limit(&parts(&within)), 5);
    }
}
