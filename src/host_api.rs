//! Seam between handlers/services and the SDK host functions.
//!
//! `gameap_plugin_sdk::host` is compiled only for wasm32, so the rest of the
//! crate depends on this narrow, domain-shaped trait instead; native
//! `cargo test` runs the real router + handlers against [`mock::MockHost`].

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostApiError {
    /// ABI/transport failure of the host call itself.
    Call(String),
    /// The daemon/panel reported a failed operation (`success: false` / `error`).
    Op(String),
}

impl HostApiError {
    pub fn message(&self) -> &str {
        match self {
            Self::Call(m) | Self::Op(m) => m,
        }
    }

    pub fn into_message(self) -> String {
        match self {
            Self::Call(m) | Self::Op(m) => m,
        }
    }
}

pub type HostResult<T> = Result<T, HostApiError>;

/// Storage entity scope (mirrors `gameap.EntityType` + entity id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageEntity {
    pub entity_type: i32,
    pub entity_id: u64,
}

/// `gameap.EntityType::Node`.
pub const ENTITY_NODE: i32 = 2;
/// `gameap.EntityType::Server`.
pub const ENTITY_SERVER: i32 = 6;

impl StorageEntity {
    pub fn node(id: u64) -> Self {
        Self {
            entity_type: ENTITY_NODE,
            entity_id: id,
        }
    }

    pub fn server(id: u64) -> Self {
        Self {
            entity_type: ENTITY_SERVER,
            entity_id: id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub id: u64,
    pub node_id: u64,
    pub name: String,
    pub game_id: String,
    pub dir: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInfo {
    pub id: u64,
    pub name: String,
    pub ips: Vec<String>,
    pub work_path: String,
    /// `linux`, `windows`, `macos` or `other` as normalized by the panel;
    /// empty on hosts predating the field.
    pub os: String,
}

impl NodeInfo {
    pub fn os_kind(&self) -> crate::domain::NodeOs {
        crate::domain::NodeOs::parse(&self.os)
    }
}

/// Result of a command executed on a node. `error` is carried but most call
/// sites ignore it, branching on `exit_code` only — that is exact Go-plugin
/// behavior (e.g. an unreachable node must read as "not installed", not a 500).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub output: String,
    pub exit_code: i32,
    pub error: Option<String>,
}

/// `gameap.DaemonTaskStatus`, narrowed to what this plugin branches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Waiting,
    Working,
    Error,
    Success,
    Canceled,
    Unknown,
}

impl TaskStatus {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Waiting,
            2 => Self::Working,
            3 => Self::Error,
            4 => Self::Success,
            5 => Self::Canceled,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonTaskInfo {
    pub id: u64,
    pub node_id: u64,
    pub status: TaskStatus,
    pub output: String,
}

/// Argon2id parameters passed to the host hasher. Must stay in sync with the
/// `security.argon2` block the install script writes for the gameap-files
/// daemon (see services::password).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2Params {
    pub memory: u32,
    pub time: u32,
    pub parallelism: u32,
    pub salt_length: u32,
    pub key_length: u32,
}

pub trait HostApi {
    fn get_server(&mut self, id: u64) -> HostResult<Option<ServerInfo>>;
    /// Game servers by filter. Empty `ids`/`node_ids` means "unfiltered on that
    /// axis" — with both empty the host returns EVERY server, unpaginated.
    /// Soft-deleted servers are never returned (the panel filters them out).
    /// Results are unordered: the host's sorting field goes raw into ORDER BY,
    /// so callers sort guest-side instead.
    fn find_servers(&mut self, ids: &[u64], node_ids: &[u64]) -> HostResult<Vec<ServerInfo>>;
    fn get_node(&mut self, id: u64) -> HostResult<Option<NodeInfo>>;
    /// Every node, unpaginated and unordered, soft-deleted ones excluded.
    fn find_nodes(&mut self) -> HostResult<Vec<NodeInfo>>;

    fn execute_command(&mut self, node_id: u64, command: &str) -> HostResult<CommandOutput>;

    fn upload(
        &mut self,
        node_id: u64,
        path: &str,
        content: &[u8],
        permissions: u32,
    ) -> HostResult<()>;
    fn download(&mut self, node_id: u64, path: &str) -> HostResult<Vec<u8>>;
    fn remove(&mut self, node_id: u64, path: &str, recursive: bool) -> HostResult<()>;

    /// Creates a CMD_EXEC daemon task; returns the task id.
    fn create_daemon_task(
        &mut self,
        node_id: u64,
        cmd: &str,
        run_after_id: Option<u64>,
    ) -> HostResult<u64>;
    fn find_daemon_task(&mut self, task_id: u64) -> HostResult<Option<DaemonTaskInfo>>;

    fn storage_get(&mut self, key: &str, entity: StorageEntity) -> HostResult<Option<Vec<u8>>>;
    fn storage_set(&mut self, key: &str, entity: StorageEntity, payload: &[u8]) -> HostResult<()>;
    fn storage_delete(&mut self, key: &str, entity: StorageEntity) -> HostResult<()>;
    /// Entries matching a key prefix within one entity scope: (key, payload).
    fn storage_list(
        &mut self,
        key_prefix: &str,
        entity: StorageEntity,
    ) -> HostResult<Vec<(String, Vec<u8>)>>;

    fn argon2_hash(&mut self, password: &str, params: Argon2Params) -> HostResult<String>;
    fn random_string(&mut self, length: i32, charset: Option<&str>) -> HostResult<String>;

    /// Wall-clock seconds since the Unix epoch (Go `time.Now().Unix()`).
    fn now_unix(&mut self) -> i64;

    /// The permissions the operator granted this plugin, read live from the
    /// panel. An `Err` means the question could not be asked at all, which is
    /// not the same answer as an empty set.
    fn plugin_grants(&mut self) -> HostResult<Vec<String>>;

    fn log_info(&mut self, message: &str);
    fn log_warn(&mut self, message: &str);
    fn log_error(&mut self, message: &str);
}

pub struct WasmHost;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use gameap_plugin_sdk::host;
    use gameap_plugin_sdk::proto::gameap::DaemonTaskType;
    use gameap_plugin_sdk::proto::gameap::plugin::sdk::{
        crypto, daemontasks, host as hostpb, nodecmd, nodefs, nodes, servers, storage,
    };

    use super::*;

    fn call_err(err: gameap_plugin_sdk::HostError) -> HostApiError {
        HostApiError::Call(err.to_string())
    }

    fn server_info(s: gameap_plugin_sdk::proto::gameap::Server) -> ServerInfo {
        ServerInfo {
            id: s.id,
            node_id: s.ds_id,
            name: s.name,
            game_id: s.game_id,
            dir: s.dir,
            enabled: s.enabled,
        }
    }

    fn node_info(n: gameap_plugin_sdk::proto::gameap::Node) -> NodeInfo {
        NodeInfo {
            id: n.id,
            name: n.name,
            ips: n.ips,
            work_path: n.work_path,
            os: n.os,
        }
    }

    impl HostApi for WasmHost {
        fn get_server(&mut self, id: u64) -> HostResult<Option<ServerInfo>> {
            let resp =
                host::servers::get_server(&servers::GetServerRequest { id }).map_err(call_err)?;
            Ok(resp.found.then_some(resp.server).flatten().map(server_info))
        }

        fn find_servers(&mut self, ids: &[u64], node_ids: &[u64]) -> HostResult<Vec<ServerInfo>> {
            // The filter stays `Some` even when every axis is empty: an absent
            // filter makes the panel's repository skip its `deleted_at IS NULL`
            // guard, so soft-deleted rows would leak in. An all-empty filter
            // narrows nothing but keeps the guard on.
            let filter = Some(servers::ServerFilter {
                ids: ids.to_vec(),
                node_ids: node_ids.to_vec(),
                game_ids: Vec::new(),
                enabled: None,
                process_active: None,
                installed: None,
            });
            let resp = host::servers::find_servers(&servers::FindServersRequest {
                filter,
                sorting: Vec::new(),
                pagination: None,
            })
            .map_err(call_err)?;
            Ok(resp.servers.into_iter().map(server_info).collect())
        }

        fn get_node(&mut self, id: u64) -> HostResult<Option<NodeInfo>> {
            let resp = host::nodes::get_node(&nodes::GetNodeRequest { id }).map_err(call_err)?;
            Ok(resp.found.then_some(resp.node).flatten().map(node_info))
        }

        fn find_nodes(&mut self) -> HostResult<Vec<NodeInfo>> {
            // Same reason as find_servers: `FindNodesRequest::default()` sends no
            // filter, and the panel's node repository then drops its
            // `deleted_at IS NULL` guard and hands back deleted nodes too.
            let resp = host::nodes::find_nodes(&nodes::FindNodesRequest {
                filter: Some(nodes::NodeFilter::default()),
                sorting: Vec::new(),
                pagination: None,
            })
            .map_err(call_err)?;
            Ok(resp.nodes.into_iter().map(node_info).collect())
        }

        fn execute_command(&mut self, node_id: u64, command: &str) -> HostResult<CommandOutput> {
            let resp = host::nodecmd::execute_command(&nodecmd::ExecuteCommandRequest {
                node_id,
                command: command.to_owned(),
                work_dir: None,
            })
            .map_err(call_err)?;
            Ok(CommandOutput {
                output: resp.output,
                exit_code: resp.exit_code,
                error: resp.error,
            })
        }

        fn upload(
            &mut self,
            node_id: u64,
            path: &str,
            content: &[u8],
            permissions: u32,
        ) -> HostResult<()> {
            let resp = host::nodefs::upload(&nodefs::UploadRequest {
                node_id,
                path: path.to_owned(),
                content: content.to_vec(),
                permissions,
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(format!(
                    "upload failed: {}",
                    resp.error.unwrap_or_default()
                )))
            }
        }

        fn download(&mut self, node_id: u64, path: &str) -> HostResult<Vec<u8>> {
            // offset/length 0 = the whole file, up to the panel's inline limit;
            // config.yaml is far below it.
            let resp = host::nodefs::download(&nodefs::DownloadRequest {
                node_id,
                path: path.to_owned(),
                offset: 0,
                length: 0,
            })
            .map_err(call_err)?;
            match resp.error {
                Some(err) if !err.is_empty() => Err(HostApiError::Op(err)),
                _ => Ok(resp.content),
            }
        }

        fn remove(&mut self, node_id: u64, path: &str, recursive: bool) -> HostResult<()> {
            let resp = host::nodefs::remove(&nodefs::RemoveRequest {
                node_id,
                path: path.to_owned(),
                recursive,
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn create_daemon_task(
            &mut self,
            node_id: u64,
            cmd: &str,
            run_after_id: Option<u64>,
        ) -> HostResult<u64> {
            let resp = host::daemontasks::create_daemon_task(&daemontasks::CreateDaemonTaskRequest {
                node_id,
                server_id: None,
                task_type: DaemonTaskType::CmdExec as i32,
                data: None,
                cmd: Some(cmd.to_owned()),
                run_after_id,
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(resp.task_id)
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn find_daemon_task(&mut self, task_id: u64) -> HostResult<Option<DaemonTaskInfo>> {
            let resp = host::daemontasks::find_daemon_tasks(&daemontasks::FindDaemonTasksRequest {
                filter: Some(daemontasks::DaemonTaskFilter {
                    ids: vec![task_id],
                    ..Default::default()
                }),
                ..Default::default()
            })
            .map_err(call_err)?;
            Ok(resp.tasks.into_iter().next().map(|t| DaemonTaskInfo {
                id: t.id,
                node_id: t.node_id,
                status: TaskStatus::from_i32(t.status),
                output: t.output.unwrap_or_default(),
            }))
        }

        fn storage_get(&mut self, key: &str, entity: StorageEntity) -> HostResult<Option<Vec<u8>>> {
            let resp = host::storage::get(&storage::StorageGetRequest {
                key: key.to_owned(),
                entity_type: Some(entity.entity_type),
                entity_id: Some(entity.entity_id),
            })
            .map_err(call_err)?;
            Ok(resp.found.then_some(resp.payload).flatten())
        }

        fn storage_set(
            &mut self,
            key: &str,
            entity: StorageEntity,
            payload: &[u8],
        ) -> HostResult<()> {
            let resp = host::storage::set(&storage::StorageSetRequest {
                key: key.to_owned(),
                entity_type: Some(entity.entity_type),
                entity_id: Some(entity.entity_id),
                payload: payload.to_vec(),
            })
            .map_err(call_err)?;
            if resp.success {
                Ok(())
            } else {
                Err(HostApiError::Op(resp.error.unwrap_or_default()))
            }
        }

        fn storage_delete(&mut self, key: &str, entity: StorageEntity) -> HostResult<()> {
            host::storage::delete(&storage::StorageDeleteRequest {
                key: key.to_owned(),
                entity_type: Some(entity.entity_type),
                entity_id: Some(entity.entity_id),
            })
            .map_err(call_err)?;
            // Go's repository ignores the response body of delete entirely.
            Ok(())
        }

        fn storage_list(
            &mut self,
            key_prefix: &str,
            entity: StorageEntity,
        ) -> HostResult<Vec<(String, Vec<u8>)>> {
            // No page window: a server's user list is small, and callers
            // expect the complete set like the Go repository returned.
            let resp = host::storage::list(&storage::StorageListRequest {
                key_prefix: Some(key_prefix.to_owned()),
                entity_type: Some(entity.entity_type),
                entity_id: Some(entity.entity_id),
                limit: None,
                offset: None,
            })
            .map_err(call_err)?;
            Ok(resp
                .entries
                .into_iter()
                .map(|e| (e.key, e.payload))
                .collect())
        }

        fn argon2_hash(&mut self, password: &str, params: Argon2Params) -> HostResult<String> {
            let resp = host::crypto::argon2_hash(&crypto::Argon2HashRequest {
                password: password.to_owned(),
                params: Some(crypto::Argon2Params {
                    memory: params.memory,
                    time: params.time,
                    parallelism: params.parallelism,
                    salt_length: params.salt_length,
                    key_length: params.key_length,
                }),
            })
            .map_err(call_err)?;
            match resp.error {
                Some(err) if !err.is_empty() => Err(HostApiError::Op(err)),
                _ => Ok(resp.hash),
            }
        }

        fn random_string(&mut self, length: i32, charset: Option<&str>) -> HostResult<String> {
            let resp = host::crypto::random_string(&crypto::RandomStringRequest {
                length,
                charset: charset.map(str::to_owned),
            })
            .map_err(call_err)?;
            match resp.error {
                Some(err) if !err.is_empty() => Err(HostApiError::Op(err)),
                _ => Ok(resp.value),
            }
        }

        fn now_unix(&mut self) -> i64 {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        }

        fn plugin_grants(&mut self) -> HostResult<Vec<String>> {
            // `get_grants` is the only gameap-host function imported, and on
            // purpose: an import the panel does not export makes the whole
            // module fail to load, and `get_config` / `report_status` are not
            // in every panel that has the module.
            let resp = host::host::get_grants(&hostpb::GetGrantsRequest {}).map_err(call_err)?;

            Ok(resp.permissions)
        }

        fn log_info(&mut self, message: &str) {
            host::log::info(message);
        }

        fn log_warn(&mut self, message: &str) {
            host::log::warn(message);
        }

        fn log_error(&mut self, message: &str) {
            host::log::error(message);
        }
    }
}

/// In-memory host used by native handler tests.
#[cfg(test)]
pub mod mock {
    use std::collections::{BTreeMap, VecDeque};

    use super::*;

    type StorageKey = (String, (i32, u64));

    pub struct MockHost {
        pub servers: BTreeMap<u64, ServerInfo>,
        pub nodes: BTreeMap<u64, NodeInfo>,
        pub storage: BTreeMap<StorageKey, Vec<u8>>,
        /// (node_id, path) → uploaded content.
        pub files: BTreeMap<(u64, String), Vec<u8>>,
        /// (node_id, path, permissions) uploads, in call order.
        pub uploads: Vec<(u64, String, u32)>,
        /// (node_id, path, recursive) removals, in call order.
        pub removed: Vec<(u64, String, bool)>,
        /// Commands passed to execute_command, in call order.
        pub commands: Vec<String>,
        /// Canned execute_command results, popped one per call.
        pub command_results: VecDeque<CommandOutput>,
        /// Commands whose string contains a needle return the paired output
        /// (checked before `command_results`).
        pub fail_on: Vec<(String, CommandOutput)>,
        /// (id, node_id, cmd, run_after_id) daemon tasks, in creation order.
        pub created_tasks: Vec<(u64, u64, String, Option<u64>)>,
        pub task_states: BTreeMap<u64, DaemonTaskInfo>,
        next_task_id: u64,
        /// (length, charset) per random_string call.
        pub random_calls: Vec<(i32, Option<String>)>,
        /// (password, params) per argon2_hash call.
        pub hash_calls: Vec<(String, Argon2Params)>,
        pub now: i64,
        /// What `plugin_grants` answers; empty means "nothing granted", which
        /// is a different thing from `plugin_grants_error` ("cannot tell").
        pub plugin_grants: Vec<String>,
        /// When set, `plugin_grants` fails — the host could not answer at all.
        pub plugin_grants_error: Option<String>,
        pub logs: Vec<String>,
    }

    impl Default for MockHost {
        fn default() -> Self {
            Self {
                servers: BTreeMap::new(),
                nodes: BTreeMap::new(),
                storage: BTreeMap::new(),
                files: BTreeMap::new(),
                uploads: Vec::new(),
                removed: Vec::new(),
                commands: Vec::new(),
                command_results: VecDeque::new(),
                fail_on: Vec::new(),
                created_tasks: Vec::new(),
                task_states: BTreeMap::new(),
                next_task_id: 101,
                random_calls: Vec::new(),
                hash_calls: Vec::new(),
                now: 1_700_000_000,
                plugin_grants: Vec::new(),
                plugin_grants_error: None,
                logs: Vec::new(),
            }
        }
    }

    impl MockHost {
        /// Node 1 ("node-1", linux, /srv/gameap) with server 3 ("cs", cstrike).
        /// The server dir is relative to the work path, as the panel stores it.
        pub fn standard() -> Self {
            let mut host = Self::default();
            host.nodes.insert(
                1,
                NodeInfo {
                    id: 1,
                    name: "node-1".into(),
                    ips: vec!["203.0.113.1".into()],
                    work_path: "/srv/gameap".into(),
                    os: "linux".into(),
                },
            );
            host.servers.insert(
                3,
                ServerInfo {
                    id: 3,
                    node_id: 1,
                    name: "cs".into(),
                    game_id: "cstrike".into(),
                    dir: "servers/cs".into(),
                    enabled: true,
                },
            );
            host
        }

        /// Adds node 7 ("win-1", windows, C:\gameap) with server 9 ("cs2").
        pub fn with_windows_node(mut self) -> Self {
            self.nodes.insert(
                7,
                NodeInfo {
                    id: 7,
                    name: "win-1".into(),
                    ips: vec!["203.0.113.7".into()],
                    work_path: r"C:\gameap".into(),
                    os: "windows".into(),
                },
            );
            self.servers.insert(
                9,
                ServerInfo {
                    id: 9,
                    node_id: 7,
                    name: "cs2".into(),
                    game_id: "cs2".into(),
                    dir: r"servers\cs2".into(),
                    enabled: true,
                },
            );
            self
        }

        pub fn with_node_os(mut self, id: u64, os: &str) -> Self {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.os = os.into();
            }
            self
        }

        pub fn push_result(&mut self, output: &str, exit_code: i32) {
            self.command_results.push_back(CommandOutput {
                output: output.into(),
                exit_code,
                error: None,
            });
        }

        pub fn set_task_state(&mut self, id: u64, status: TaskStatus, output: &str) {
            let node_id = self
                .created_tasks
                .iter()
                .find(|(tid, ..)| *tid == id)
                .map(|(_, n, ..)| *n)
                .unwrap_or(0);
            self.task_states.insert(
                id,
                DaemonTaskInfo {
                    id,
                    node_id,
                    status,
                    output: output.into(),
                },
            );
        }

        pub fn file(&self, node_id: u64, path: &str) -> Option<&[u8]> {
            self.files
                .get(&(node_id, path.to_string()))
                .map(Vec::as_slice)
        }

        pub fn storage_raw(&self, key: &str, entity: StorageEntity) -> Option<&[u8]> {
            self.storage
                .get(&Self::skey(key, entity))
                .map(Vec::as_slice)
        }

        pub fn seed_storage(&mut self, key: &str, entity: StorageEntity, payload: &[u8]) {
            self.storage
                .insert(Self::skey(key, entity), payload.to_vec());
        }

        fn skey(key: &str, entity: StorageEntity) -> StorageKey {
            (key.to_string(), (entity.entity_type, entity.entity_id))
        }
    }

    impl HostApi for MockHost {
        fn get_server(&mut self, id: u64) -> HostResult<Option<ServerInfo>> {
            Ok(self.servers.get(&id).cloned())
        }

        fn find_servers(&mut self, ids: &[u64], node_ids: &[u64]) -> HostResult<Vec<ServerInfo>> {
            Ok(self
                .servers
                .values()
                .filter(|s| ids.is_empty() || ids.contains(&s.id))
                .filter(|s| node_ids.is_empty() || node_ids.contains(&s.node_id))
                .cloned()
                .collect())
        }

        fn get_node(&mut self, id: u64) -> HostResult<Option<NodeInfo>> {
            Ok(self.nodes.get(&id).cloned())
        }

        fn find_nodes(&mut self) -> HostResult<Vec<NodeInfo>> {
            Ok(self.nodes.values().cloned().collect())
        }

        fn execute_command(&mut self, _node_id: u64, command: &str) -> HostResult<CommandOutput> {
            self.commands.push(command.to_string());
            for (needle, out) in &self.fail_on {
                if command.contains(needle.as_str()) {
                    return Ok(out.clone());
                }
            }
            Ok(self.command_results.pop_front().unwrap_or(CommandOutput {
                output: String::new(),
                exit_code: 0,
                error: None,
            }))
        }

        fn upload(
            &mut self,
            node_id: u64,
            path: &str,
            content: &[u8],
            permissions: u32,
        ) -> HostResult<()> {
            self.files
                .insert((node_id, path.to_string()), content.to_vec());
            self.uploads.push((node_id, path.to_string(), permissions));
            Ok(())
        }

        fn download(&mut self, node_id: u64, path: &str) -> HostResult<Vec<u8>> {
            match self.files.get(&(node_id, path.to_string())) {
                Some(content) => Ok(content.clone()),
                None => Err(HostApiError::Op(format!("file not found: {path}"))),
            }
        }

        fn remove(&mut self, node_id: u64, path: &str, recursive: bool) -> HostResult<()> {
            self.removed.push((node_id, path.to_string(), recursive));
            self.files.remove(&(node_id, path.to_string()));
            Ok(())
        }

        fn create_daemon_task(
            &mut self,
            node_id: u64,
            cmd: &str,
            run_after_id: Option<u64>,
        ) -> HostResult<u64> {
            let id = self.next_task_id;
            self.next_task_id += 1;
            self.created_tasks
                .push((id, node_id, cmd.to_string(), run_after_id));
            self.task_states.insert(
                id,
                DaemonTaskInfo {
                    id,
                    node_id,
                    status: TaskStatus::Waiting,
                    output: String::new(),
                },
            );
            Ok(id)
        }

        fn find_daemon_task(&mut self, task_id: u64) -> HostResult<Option<DaemonTaskInfo>> {
            Ok(self.task_states.get(&task_id).cloned())
        }

        fn storage_get(&mut self, key: &str, entity: StorageEntity) -> HostResult<Option<Vec<u8>>> {
            Ok(self.storage.get(&Self::skey(key, entity)).cloned())
        }

        fn storage_set(
            &mut self,
            key: &str,
            entity: StorageEntity,
            payload: &[u8],
        ) -> HostResult<()> {
            self.storage
                .insert(Self::skey(key, entity), payload.to_vec());
            Ok(())
        }

        fn storage_delete(&mut self, key: &str, entity: StorageEntity) -> HostResult<()> {
            self.storage.remove(&Self::skey(key, entity));
            Ok(())
        }

        fn storage_list(
            &mut self,
            key_prefix: &str,
            entity: StorageEntity,
        ) -> HostResult<Vec<(String, Vec<u8>)>> {
            let want = (entity.entity_type, entity.entity_id);
            Ok(self
                .storage
                .iter()
                .filter(|((key, ent), _)| *ent == want && key.starts_with(key_prefix))
                .map(|((key, _), payload)| (key.clone(), payload.clone()))
                .collect())
        }

        fn argon2_hash(&mut self, password: &str, params: Argon2Params) -> HostResult<String> {
            self.hash_calls.push((password.to_string(), params));
            Ok(format!("$argon2id$mock${password}"))
        }

        fn random_string(&mut self, length: i32, charset: Option<&str>) -> HostResult<String> {
            self.random_calls
                .push((length, charset.map(str::to_string)));
            Ok("MOCKPASSWORD1234".into())
        }

        fn now_unix(&mut self) -> i64 {
            self.now
        }

        fn plugin_grants(&mut self) -> HostResult<Vec<String>> {
            match &self.plugin_grants_error {
                Some(err) => Err(HostApiError::Call(err.clone())),
                None => Ok(self.plugin_grants.clone()),
            }
        }

        fn log_info(&mut self, message: &str) {
            self.logs.push(format!("INFO {message}"));
        }

        fn log_warn(&mut self, message: &str) {
            self.logs.push(format!("WARN {message}"));
        }

        fn log_error(&mut self, message: &str) {
            self.logs.push(format!("ERROR {message}"));
        }
    }
}
