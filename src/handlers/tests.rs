//! Dispatch-level tests: real `pb::HttpRequest` values through
//! `router::dispatch` against the in-memory `MockHost`, asserting status
//! codes, JSON bodies, storage contents and node-side effects. These pin the
//! wire contract inherited from the Go plugin.

use std::collections::HashMap;

use gameap_plugin_sdk::proto::gameap as gameap_pb;
use gameap_plugin_sdk::proto::gameap::plugin as pb;
use serde_json::{Value, json};

use crate::domain::{FtpUser, SetupStatus};
use crate::host_api::mock::MockHost;
use crate::host_api::{HostApi, StorageEntity, TaskStatus};
use crate::router;
use crate::services::password::ARGON2_PARAMS;
use crate::services::store;

fn request(method: &str, path: &str, body: &[u8]) -> pb::HttpRequest {
    pb::HttpRequest {
        method: method.into(),
        path: path.into(),
        body: body.to_vec(),
        ..Default::default()
    }
}

fn request_with_query(method: &str, path: &str, query: &[(&str, &str)]) -> pb::HttpRequest {
    let mut req = request(method, path, b"");
    let mut map = HashMap::new();
    for (key, value) in query {
        map.insert(
            key.to_string(),
            pb::QueryParamValues {
                values: vec![value.to_string()],
            },
        );
    }
    req.query_params = map;
    req
}

fn dispatch(host: &mut MockHost, req: &pb::HttpRequest) -> (i32, Value) {
    let resp = router::dispatch(host, req);
    let body: Value = if resp.body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&resp.body).expect("response body is JSON")
    };
    (resp.status_code, body)
}

fn create_user(host: &mut MockHost, server_id: u64, body: Value) -> (i32, Value) {
    dispatch(
        host,
        &request(
            "POST",
            &format!("/servers/{server_id}/ftp-users"),
            body.to_string().as_bytes(),
        ),
    )
}

fn stored_user(host: &MockHost, server_id: u64, username: &str) -> FtpUser {
    let raw = host
        .storage_raw(&store::user_key(username), StorageEntity::server(server_id))
        .expect("user stored");
    serde_json::from_slice(raw).expect("stored user parses")
}

// --- users CRUD ---

#[test]
fn create_user_with_generated_password() {
    let mut host = MockHost::standard();
    let (status, body) = create_user(&mut host, 3, json!({"username": "bob"}));
    assert_eq!(status, 201);
    assert_eq!(body["username"], "bob");
    assert_eq!(body["password"], "MOCKPASSWORD1234");
    assert_eq!(body["home_dir"], "/srv/gameap/servers/cs");
    assert_eq!(body["enabled"], true);

    assert_eq!(host.random_calls.len(), 1);
    assert_eq!(host.random_calls[0].0, 16);
    assert_eq!(
        host.random_calls[0].1.as_deref(),
        Some("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
    );
    assert_eq!(host.hash_calls, vec![("MOCKPASSWORD1234".into(), ARGON2_PARAMS)]);

    let user = stored_user(&host, 3, "bob");
    assert_eq!(user.password_hash, "$argon2id$mock$MOCKPASSWORD1234");
    assert_eq!(user.access_rules, crate::domain::default_access_rules());
    assert!(user.enabled);

    let list_raw = host
        .storage_raw(store::KEY_SERVER_USER_LIST, StorageEntity::server(3))
        .expect("user list stored");
    assert_eq!(String::from_utf8_lossy(list_raw), r#"["bob"]"#);

    let yaml = host
        .file(1, "/etc/gameap-files/users.d/bob.yaml")
        .expect("yaml synced to node");
    let synced: FtpUser = serde_yaml_ng::from_slice(yaml).expect("valid yaml");
    assert_eq!(synced, user);
    assert_eq!(
        host.uploads,
        vec![(1, "/etc/gameap-files/users.d/bob.yaml".to_string(), 0o600)]
    );
}

#[test]
fn create_user_with_explicit_password() {
    let mut host = MockHost::standard();
    let (status, body) = create_user(
        &mut host,
        3,
        json!({
            "username": "alice",
            "password": "s3cret",
            "home_dir": "/custom",
            "quota_bytes": 1024,
            "enabled": false,
            "description": "temp"
        }),
    );
    assert_eq!(status, 201);
    assert!(body.get("password").is_none(), "no plaintext echo: {body}");
    assert_eq!(body["home_dir"], "/custom");
    assert_eq!(body["quota_bytes"], 1024);
    assert_eq!(body["enabled"], false);
    assert!(host.random_calls.is_empty());
    assert_eq!(host.hash_calls[0].0, "s3cret");
}

#[test]
fn create_user_error_cases() {
    let mut host = MockHost::standard();

    let (status, body) = create_user(&mut host, 3, json!({"username": ""}));
    assert_eq!((status, body["message"].as_str().unwrap()), (400, "username is required"));

    let (status, body) = create_user(&mut host, 3, json!({"username": "1bad"}));
    assert_eq!(status, 400);
    assert_eq!(
        body["message"],
        "invalid username: must be alphanumeric with underscores, 3-32 chars"
    );

    let (status, body) = dispatch(&mut host, &request("POST", "/servers/3/ftp-users", b"{oops"));
    assert_eq!((status, body["message"].as_str().unwrap()), (400, "invalid request body"));

    let (status, body) = create_user(&mut host, 99, json!({"username": "bob"}));
    assert_eq!(status, 500);
    assert_eq!(body["code"], "INTERNAL_ERROR");
    assert_eq!(body["message"], "server 99 not found");

    create_user(&mut host, 3, json!({"username": "bob"}));
    let (status, body) = create_user(&mut host, 3, json!({"username": "bob"}));
    assert_eq!(status, 409);
    assert_eq!(body["code"], "CONFLICT");
    assert_eq!(body["message"], "user already exists");
}

#[test]
fn list_and_get_users() {
    let mut host = MockHost::standard();
    create_user(&mut host, 3, json!({"username": "bob"}));
    create_user(&mut host, 3, json!({"username": "alice", "password": "x"}));

    let (status, body) = dispatch(&mut host, &request("GET", "/servers/3/ftp-users", b""));
    assert_eq!(status, 200);
    let list = body.as_array().expect("array response");
    assert_eq!(list.len(), 2);
    for entry in list {
        assert!(entry["ssh_keys_count"].is_number());
        assert!(entry["access_rules"].is_array());
        assert!(entry["virtual_paths"].is_array());
        assert!(entry.get("password_hash").is_none(), "hash must not leak");
    }

    let (status, body) = dispatch(&mut host, &request("GET", "/servers/3/ftp-users/bob", b""));
    assert_eq!(status, 200);
    assert_eq!(body["username"], "bob");

    let (status, body) = dispatch(&mut host, &request("GET", "/servers/3/ftp-users/ghost", b""));
    assert_eq!(status, 404);
    assert_eq!(body["message"], "user not found");

    let (status, body) = dispatch(&mut host, &request("GET", "/servers/abc/ftp-users", b""));
    assert_eq!(status, 400);
    assert_eq!(body["message"], "invalid serverId: must be a number");
}

#[test]
fn update_user_ignores_empty_password() {
    let mut host = MockHost::standard();
    create_user(&mut host, 3, json!({"username": "bob", "password": "orig"}));
    let hash_before = stored_user(&host, 3, "bob").password_hash;

    let (status, body) = dispatch(
        &mut host,
        &request(
            "PUT",
            "/servers/3/ftp-users/bob",
            json!({"password": "", "description": "updated", "quota_bytes": 42})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(status, 200);
    assert_eq!(body["description"], "updated");
    assert_eq!(body["quota_bytes"], 42);

    let user = stored_user(&host, 3, "bob");
    assert_eq!(user.password_hash, hash_before, "empty password must be ignored");
    assert_eq!(host.uploads.len(), 2, "yaml re-synced after update");

    let (status, _) = dispatch(
        &mut host,
        &request("PUT", "/servers/99/ftp-users/bob", b"{}"),
    );
    assert_eq!(status, 500, "missing server is a 500 on update (Go parity)");
}

#[test]
fn delete_user_cleans_storage_and_node() {
    let mut host = MockHost::standard();
    create_user(&mut host, 3, json!({"username": "bob"}));

    let (status, body) = dispatch(&mut host, &request("DELETE", "/servers/3/ftp-users/bob", b""));
    assert_eq!(status, 200);
    assert_eq!(body, json!({"deleted": true}));

    assert!(host
        .storage_raw(&store::user_key("bob"), StorageEntity::server(3))
        .is_none());
    let list_raw = host
        .storage_raw(store::KEY_SERVER_USER_LIST, StorageEntity::server(3))
        .expect("list key kept");
    assert_eq!(String::from_utf8_lossy(list_raw), "[]");
    assert_eq!(
        host.removed,
        vec![(1, "/etc/gameap-files/users.d/bob.yaml".to_string(), false)]
    );

    let (status, _) = dispatch(&mut host, &request("DELETE", "/servers/3/ftp-users/bob", b""));
    assert_eq!(status, 404);
}

// --- access rules / virtual paths ---

#[test]
fn access_rules_round_trip() {
    let mut host = MockHost::standard();
    create_user(&mut host, 3, json!({"username": "bob"}));

    let (status, body) = dispatch(
        &mut host,
        &request("GET", "/servers/3/ftp-users/bob/access-rules", b""),
    );
    assert_eq!(status, 200);
    assert_eq!(body["rules"][0]["path"], "/**");
    assert_eq!(body["rules"][0]["permissions"], json!(["read", "write", "delete", "list"]));

    let (status, body) = dispatch(
        &mut host,
        &request(
            "PUT",
            "/servers/3/ftp-users/bob/access-rules",
            json!({"rules": [{"path": "/data", "permissions": ["read", "list"]}]})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(status, 200);
    assert_eq!(body["rules"].as_array().unwrap().len(), 1);
    assert_eq!(stored_user(&host, 3, "bob").access_rules.len(), 1);

    let (status, body) = dispatch(
        &mut host,
        &request(
            "PUT",
            "/servers/3/ftp-users/bob/access-rules",
            br#"{"rules": null}"#,
        ),
    );
    assert_eq!(status, 200);
    assert_eq!(body["rules"], json!([]));

    let (status, _) = dispatch(
        &mut host,
        &request("GET", "/servers/3/ftp-users/ghost/access-rules", b""),
    );
    assert_eq!(status, 404);
}

#[test]
fn virtual_paths_round_trip() {
    let mut host = MockHost::standard();
    create_user(&mut host, 3, json!({"username": "bob"}));

    let (status, body) = dispatch(
        &mut host,
        &request(
            "PUT",
            "/servers/3/ftp-users/bob/virtual-paths",
            json!({"paths": [{"virtual": "/shared", "target": "/srv/shared", "permissions": ["read"], "read_only": true}]})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(status, 200);
    assert_eq!(body["paths"][0]["virtual"], "/shared");
    assert_eq!(body["paths"][0]["read_only"], true);

    let (status, body) = dispatch(
        &mut host,
        &request("GET", "/servers/3/ftp-users/bob/virtual-paths", b""),
    );
    assert_eq!(status, 200);
    assert_eq!(body["paths"][0]["target"], "/srv/shared");

    let user = stored_user(&host, 3, "bob");
    assert_eq!(user.virtual_paths[0].virtual_path, "/shared");
}

// --- ssh keys ---

#[test]
fn ssh_keys_lifecycle() {
    let mut host = MockHost::standard();
    create_user(&mut host, 3, json!({"username": "bob"}));

    let (status, body) = dispatch(
        &mut host,
        &request("GET", "/servers/3/ftp-users/bob/ssh-keys", b""),
    );
    assert_eq!((status, body.clone()), (200, json!({"keys": []})));

    let (status, body) = dispatch(
        &mut host,
        &request(
            "POST",
            "/servers/3/ftp-users/bob/ssh-keys",
            json!({"key": "ssh-ed25519 AAAAC3Nza bob@pc"}).to_string().as_bytes(),
        ),
    );
    assert_eq!(status, 201);
    assert_eq!(body["keys"].as_array().unwrap().len(), 1);

    let (status, body) = dispatch(
        &mut host,
        &request("POST", "/servers/3/ftp-users/bob/ssh-keys", br#"{"key": "garbage"}"#),
    );
    assert_eq!(status, 400);
    assert_eq!(body["message"], "invalid SSH public key format");

    let (status, body) = dispatch(
        &mut host,
        &request("POST", "/servers/3/ftp-users/bob/ssh-keys", br#"{"key": ""}"#),
    );
    assert_eq!((status, body["message"].as_str().unwrap()), (400, "key is required"));

    // Out-of-range index is a 500 (Go parity), non-numeric is a 400.
    let (status, body) = dispatch(
        &mut host,
        &request("DELETE", "/servers/3/ftp-users/bob/ssh-keys/5", b""),
    );
    assert_eq!(status, 500);
    assert_eq!(body["message"], "invalid key index");

    let (status, body) = dispatch(
        &mut host,
        &request("DELETE", "/servers/3/ftp-users/bob/ssh-keys/abc", b""),
    );
    assert_eq!(status, 400);
    assert_eq!(body["message"], "invalid index: must be a number");

    let (status, body) = dispatch(
        &mut host,
        &request("DELETE", "/servers/3/ftp-users/bob/ssh-keys/0", b""),
    );
    assert_eq!(status, 200);
    assert_eq!(body, json!({"keys": []}));
}

// --- node setup / status / config ---

#[test]
fn setup_creates_chained_tasks_and_stores_state() {
    let mut host = MockHost::standard();
    let (status, body) = dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));
    assert_eq!(status, 200);
    assert_eq!(body["status"], "installing");
    assert_eq!(body["task_id"], 102);
    assert!(body.get("started_at").is_none(), "DTO must not expose started_at");

    assert_eq!(host.created_tasks.len(), 2);
    let (download_id, node_id, download_cmd, run_after) = &host.created_tasks[0];
    assert_eq!((*download_id, *node_id, *run_after), (101, 1, None));
    assert_eq!(
        download_cmd,
        "get-tool https://raw.githubusercontent.com/gameap/scripts/master/ftp/gameap-files/install-files-linux.sh"
    );
    let (install_id, _, install_cmd, run_after) = &host.created_tasks[1];
    assert_eq!((*install_id, *run_after), (102, Some(101)));
    assert!(install_cmd.starts_with("install-files-linux.sh --data-dir=/srv/gameap "));

    let raw = host
        .storage_raw(store::KEY_NODE_SETUP_STATUS, StorageEntity::node(1))
        .expect("status stored");
    let stored: Value = serde_json::from_slice(raw).unwrap();
    assert_eq!(stored["status"], "installing");
    assert_eq!(stored["task_id"], 102);
    assert_eq!(stored["download_task_id"], 101);
    assert_eq!(stored["started_at"], 1_700_000_000_i64);

    let config_raw = host
        .storage_raw(store::KEY_NODE_CONFIG, StorageEntity::node(1))
        .expect("config stored");
    let config: Value = serde_json::from_slice(config_raw).unwrap();
    assert_eq!(config["ftp"]["port"], 21);
    assert_eq!(config["sftp"]["port"], 2222);

    // Re-running setup while installing must not create new tasks.
    let (status, body) = dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));
    assert_eq!((status, body["status"].as_str().unwrap()), (200, "installing"));
    assert_eq!(host.created_tasks.len(), 2);
}

#[test]
fn setup_with_overrides_and_missing_node() {
    let mut host = MockHost::standard();
    let (status, _) = dispatch(
        &mut host,
        &request(
            "POST",
            "/nodes/1/setup",
            json!({"ftp": {"port": 2121, "tls_enabled": true}, "sftp": {"port": 2323}})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(status, 200);
    let install_cmd = &host.created_tasks[1].2;
    assert!(install_cmd.contains("--ftp-listen-address=:2121"), "{install_cmd}");
    assert!(install_cmd.contains("--ftp-tls-enabled=true"));
    assert!(install_cmd.contains("--sftp-listen-address=:2323"));

    let (status, body) = dispatch(&mut host, &request("POST", "/nodes/99/setup", b""));
    assert_eq!(status, 500);
    assert_eq!(body["message"], "node 99 not found");
}

#[test]
fn status_probes_node_when_nothing_stored() {
    let mut host = MockHost::standard();
    host.push_result("gameap-files version v1.2.3\ncommit abc", 0);
    let (status, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(status, 200);
    assert_eq!(body["status"], "installed");
    assert_eq!(body["version"], "v1.2.3");
    assert!(
        host.storage_raw(store::KEY_NODE_SETUP_STATUS, StorageEntity::node(1)).is_none(),
        "probe result is not persisted (Go parity)"
    );

    host.push_result("sh: gameap-files: command not found", 0);
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "not_installed");

    host.push_result("", 127);
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "not_installed");

    host.push_result("no semver here", 0);
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "not_installed");
}

#[test]
fn status_follows_install_task_lifecycle() {
    let mut host = MockHost::standard();
    dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));

    // Task still waiting → installing.
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "installing");

    // Task succeeded → verified via version probe → installed, persisted.
    host.set_task_state(102, TaskStatus::Success, "done");
    host.push_result("gameap-files version 1.0.0", 0);
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "installed");
    assert_eq!(body["version"], "1.0.0");
    let stored = store::get_status(&mut host, 1).unwrap().unwrap();
    assert_eq!(stored.status, SetupStatus::Installed);
}

#[test]
fn status_reports_task_failure_and_cancellation() {
    let mut host = MockHost::standard();
    dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));
    host.set_task_state(102, TaskStatus::Error, "curl: (6) could not resolve");
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error_message"], "curl: (6) could not resolve");

    let mut host = MockHost::standard();
    dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));
    host.set_task_state(102, TaskStatus::Canceled, "");
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error_message"], "installation task was canceled");
}

#[test]
fn status_times_out_stuck_installations() {
    let mut host = MockHost::standard();
    host.seed_storage(
        store::KEY_NODE_SETUP_STATUS,
        StorageEntity::node(1),
        json!({"status": "installing", "task_id": 555, "last_check": 1_699_999_000, "started_at": 1_699_999_000})
            .to_string()
            .as_bytes(),
    );
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error_message"], "installation timed out - event may have been lost");

    // Legacy doc without started_at falls back to last_check.
    let mut host = MockHost::standard();
    host.seed_storage(
        store::KEY_NODE_SETUP_STATUS,
        StorageEntity::node(1),
        json!({"status": "installing", "task_id": 555, "last_check": 1_699_999_000})
            .to_string()
            .as_bytes(),
    );
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "failed");

    // Not yet timed out → still installing.
    let mut host = MockHost::standard();
    host.seed_storage(
        store::KEY_NODE_SETUP_STATUS,
        StorageEntity::node(1),
        json!({"status": "installing", "task_id": 555, "last_check": 1_699_999_500, "started_at": 1_699_999_500})
            .to_string()
            .as_bytes(),
    );
    let (_, body) = dispatch(&mut host, &request("GET", "/nodes/1/status", b""));
    assert_eq!(body["status"], "installing");
}

const DAEMON_CONFIG: &str = "server:\n  data_dir: /srv/gameap\nftp:\n  enabled: true\n  listen_addr: \":21\"\n  tls:\n    cert_file: /etc/gameap-files/cert.pem\nsecurity:\n  argon2:\n    memory: 65536\n";

#[test]
fn node_config_get_and_update() {
    let mut host = MockHost::standard();

    let (status, body) = dispatch(&mut host, &request("GET", "/nodes/1/config", b""));
    assert_eq!(status, 200);
    assert_eq!(body["ftp"]["port"], 21);
    assert_eq!(body["ftp"]["passive_port_min"], 30000);
    assert_eq!(body["sftp"]["port"], 2222);

    host.upload(1, "/etc/gameap-files/config.yaml", DAEMON_CONFIG.as_bytes(), 0o644)
        .unwrap();
    host.uploads.clear();

    let (status, body) = dispatch(
        &mut host,
        &request(
            "PUT",
            "/nodes/1/config",
            json!({"ftp": {"port": 2121, "public_host": "ftp.example.com"}})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(status, 200);
    assert_eq!(body["ftp"]["port"], 2121);
    assert_eq!(body["ftp"]["public_host"], "ftp.example.com");
    assert_eq!(body["ftp"]["passive_port_min"], 30000, "merged with defaults");

    // Stored config reflects the merge.
    let stored: Value = serde_json::from_slice(
        host.storage_raw(store::KEY_NODE_CONFIG, StorageEntity::node(1)).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["ftp"]["port"], 2121);

    // Node config.yaml was patched, preserving foreign keys, and the service restarted.
    let patched = host.file(1, "/etc/gameap-files/config.yaml").unwrap();
    let patched: serde_yaml_ng::Value = serde_yaml_ng::from_slice(patched).unwrap();
    assert_eq!(patched["ftp"]["listen_addr"], serde_yaml_ng::Value::from(":2121"));
    assert_eq!(
        patched["ftp"]["tls"]["cert_file"],
        serde_yaml_ng::Value::from("/etc/gameap-files/cert.pem")
    );
    assert_eq!(
        patched["security"]["argon2"]["memory"],
        serde_yaml_ng::Value::from(65536)
    );
    assert_eq!(patched["server"]["data_dir"], serde_yaml_ng::Value::from("/srv/gameap"));
    assert!(host
        .commands
        .iter()
        .any(|cmd| cmd == "systemctl restart gameap-files.service"));

    // Empty body on PUT is a 400 (Go behavior differs from setup here).
    let (status, body) = dispatch(&mut host, &request("PUT", "/nodes/1/config", b""));
    assert_eq!(status, 400);
    assert!(body["message"].as_str().unwrap().starts_with("invalid request body: "));
}

#[test]
fn node_config_update_fails_when_restart_fails() {
    let mut host = MockHost::standard();
    host.upload(1, "/etc/gameap-files/config.yaml", DAEMON_CONFIG.as_bytes(), 0o644)
        .unwrap();
    host.fail_on.push((
        "systemctl restart".into(),
        crate::host_api::CommandOutput {
            output: "unit not found".into(),
            exit_code: 1,
            error: None,
        },
    ));
    let (status, body) = dispatch(
        &mut host,
        &request("PUT", "/nodes/1/config", br#"{"ftp": {"port": 2121}}"#),
    );
    assert_eq!(status, 500);
    assert_eq!(
        body["message"],
        "failed to apply config to node: restart failed: unit not found"
    );
}

// --- admin ---

#[test]
fn admin_nodes_lists_status_per_node() {
    let mut host = MockHost::standard();
    host.nodes.insert(
        2,
        crate::host_api::NodeInfo {
            id: 2,
            name: "node-2".into(),
            ips: vec![],
            work_path: "/srv".into(),
        },
    );
    host.seed_storage(
        store::KEY_NODE_SETUP_STATUS,
        StorageEntity::node(1),
        br#"{"status":"installed","version":"v1.0.0","last_check":1700000000}"#,
    );

    let (status, body) = dispatch(&mut host, &request("GET", "/admin/nodes", b""));
    assert_eq!(status, 200);
    let nodes = body["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0]["id"], 1);
    assert_eq!(nodes[0]["ip"], "203.0.113.1");
    assert_eq!(nodes[0]["plugin_status"]["status"], "installed");
    assert_eq!(nodes[0]["plugin_status"]["version"], "v1.0.0");
    assert_eq!(nodes[1]["ip"], "");
    assert_eq!(nodes[1]["plugin_status"], json!({"status": "not_installed"}));
}

#[test]
fn admin_users_groups_and_filters() {
    let mut host = MockHost::standard();
    host.nodes.insert(
        2,
        crate::host_api::NodeInfo {
            id: 2,
            name: "node-2".into(),
            ips: vec![],
            work_path: "/srv".into(),
        },
    );
    host.servers.insert(
        5,
        crate::host_api::ServerInfo {
            id: 5,
            node_id: 2,
            name: "mc".into(),
            game_id: "minecraft".into(),
            dir: "/srv/mc".into(),
            enabled: true,
        },
    );
    create_user(&mut host, 3, json!({"username": "bob"}));
    create_user(&mut host, 3, json!({"username": "alice", "enabled": false}));
    create_user(&mut host, 5, json!({"username": "steve"}));

    let (status, body) = dispatch(&mut host, &request("GET", "/admin/users", b""));
    assert_eq!(status, 200);
    assert_eq!(body["total"], 3);
    let users = body["grouped"]["1"]["servers"]["3"]["users"].as_array().unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(body["grouped"]["1"]["node_name"], "node-1");
    assert_eq!(body["grouped"]["2"]["servers"]["5"]["server_name"], "mc");
    assert_eq!(body["grouped"]["2"]["servers"]["5"]["game_id"], "minecraft");

    let (_, body) = dispatch(&mut host, &request_with_query("GET", "/admin/users", &[("search", "BO")]));
    assert_eq!(body["total"], 1);

    let (_, body) = dispatch(&mut host, &request_with_query("GET", "/admin/users", &[("node_id", "2")]));
    assert_eq!(body["total"], 1);
    assert!(body["grouped"].get("1").is_none());

    let (_, body) = dispatch(&mut host, &request_with_query("GET", "/admin/users", &[("server_id", "3")]));
    assert_eq!(body["total"], 2);

    let (_, body) = dispatch(&mut host, &request_with_query("GET", "/admin/users", &[("enabled", "true")]));
    assert_eq!(body["total"], 2);

    // Go quirk: any non-"true" value filters for disabled users.
    let (_, body) = dispatch(&mut host, &request_with_query("GET", "/admin/users", &[("enabled", "banana")]));
    assert_eq!(body["total"], 1);
}

// --- events ---

fn task_event(event_type: pb::EventType, node_id: u64, task_id: u64, task_type: &str) -> pb::Event {
    pb::Event {
        r#type: event_type as i32,
        payload: Some(pb::event::Payload::TaskEvent(pb::TaskEventPayload {
            task_id,
            node_id,
            server_id: None,
            task_type: task_type.into(),
            status: String::new(),
            extra_data: HashMap::new(),
        })),
        ..Default::default()
    }
}

fn server_deleted_event(server_id: u64, ds_id: u64) -> pb::Event {
    pb::Event {
        r#type: pb::EventType::ServerDeleted as i32,
        payload: Some(pb::event::Payload::ServerEvent(pb::ServerEventPayload {
            server: Some(gameap_pb::Server {
                id: server_id,
                ds_id,
                ..Default::default()
            }),
            extra_data: HashMap::new(),
        })),
        ..Default::default()
    }
}

#[test]
fn server_deleted_wipes_storage_and_node_files() {
    let mut host = MockHost::standard();
    create_user(&mut host, 3, json!({"username": "bob"}));
    create_user(&mut host, 3, json!({"username": "alice"}));
    host.removed.clear();
    // Simulate the panel having already deleted the server row.
    host.servers.remove(&3);

    let result = crate::handlers::events::handle(&mut host, &server_deleted_event(3, 1));
    assert!(result.handled);

    assert!(host
        .storage_raw(&store::user_key("bob"), StorageEntity::server(3))
        .is_none());
    assert!(host
        .storage_raw(&store::user_key("alice"), StorageEntity::server(3))
        .is_none());
    assert_eq!(
        String::from_utf8_lossy(
            host.storage_raw(store::KEY_SERVER_USER_LIST, StorageEntity::server(3)).unwrap()
        ),
        "[]"
    );
    let mut removed_paths: Vec<&str> = host.removed.iter().map(|(_, p, _)| p.as_str()).collect();
    removed_paths.sort_unstable();
    assert_eq!(
        removed_paths,
        vec![
            "/etc/gameap-files/users.d/alice.yaml",
            "/etc/gameap-files/users.d/bob.yaml"
        ]
    );
    assert!(host.removed.iter().all(|(node, ..)| *node == 1), "ds_id from payload");

    let result = crate::handlers::events::handle(
        &mut host,
        &pb::Event {
            r#type: pb::EventType::ServerDeleted as i32,
            ..Default::default()
        },
    );
    assert!(!result.handled, "payload-less event is not ours");
}

#[test]
fn task_events_match_stored_task_ids() {
    let mut host = MockHost::standard();
    dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));

    // Foreign task type / unknown task id / unknown node → not handled.
    let event = task_event(pb::EventType::DaemonTaskCompleted, 1, 102, "server_start");
    assert!(!crate::handlers::events::handle(&mut host, &event).handled);
    let event = task_event(pb::EventType::DaemonTaskCompleted, 1, 999, "cmdexec");
    assert!(!crate::handlers::events::handle(&mut host, &event).handled);
    let event = task_event(pb::EventType::DaemonTaskCompleted, 2, 102, "cmdexec");
    assert!(!crate::handlers::events::handle(&mut host, &event).handled);

    // Download task completion is acknowledged but changes nothing.
    let event = task_event(pb::EventType::DaemonTaskCompleted, 1, 101, "cmdexec");
    assert!(crate::handlers::events::handle(&mut host, &event).handled);
    assert_eq!(
        store::get_status(&mut host, 1).unwrap().unwrap().status,
        SetupStatus::Installing
    );

    // Install task completion verifies and persists "installed".
    host.push_result("gameap-files version v1.0.0", 0);
    let event = task_event(pb::EventType::DaemonTaskCompleted, 1, 102, "cmdexec");
    assert!(crate::handlers::events::handle(&mut host, &event).handled);
    let stored = store::get_status(&mut host, 1).unwrap().unwrap();
    assert_eq!(stored.status, SetupStatus::Installed);
    assert_eq!(stored.version, "v1.0.0");

    // Once no longer installing, further task events are ignored.
    let event = task_event(pb::EventType::DaemonTaskCompleted, 1, 102, "cmdexec");
    assert!(!crate::handlers::events::handle(&mut host, &event).handled);
}

#[test]
fn task_failure_records_output_from_task() {
    let mut host = MockHost::standard();
    dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));
    host.set_task_state(102, TaskStatus::Error, "install.sh: exit status 1");

    let event = task_event(pb::EventType::DaemonTaskFailed, 1, 102, "cmdexec");
    assert!(crate::handlers::events::handle(&mut host, &event).handled);
    let stored = store::get_status(&mut host, 1).unwrap().unwrap();
    assert_eq!(stored.status, SetupStatus::Failed);
    assert_eq!(stored.error_message, "install.sh: exit status 1");
    assert_eq!(stored.task_id, 102);

    // Download-task failure also fails the install, keeping the install task id.
    let mut host = MockHost::standard();
    dispatch(&mut host, &request("POST", "/nodes/1/setup", b""));
    let event = task_event(pb::EventType::DaemonTaskFailed, 1, 101, "cmdexec");
    assert!(crate::handlers::events::handle(&mut host, &event).handled);
    let stored = store::get_status(&mut host, 1).unwrap().unwrap();
    assert_eq!(stored.status, SetupStatus::Failed);
    assert_eq!(stored.error_message, "installation task failed");
    assert_eq!(stored.task_id, 102);
}

// --- fallback ---

#[test]
fn unknown_route_is_404() {
    let mut host = MockHost::standard();
    let (status, body) = dispatch(&mut host, &request("GET", "/nope", b""));
    assert_eq!(status, 404);
    assert_eq!(body, json!({"code": "NOT_FOUND", "message": "route not found"}));
}
