//! Event handling.
//!
//! SERVER_DELETED wipes the server's users from storage and their YAML
//! drop-ins from the node — the node id comes from the event payload's
//! `ds_id`, because the server row is already deleted and can no longer be
//! resolved via get_server (the Go version tried exactly that and silently
//! orphaned node files).
//!
//! DAEMON_TASK_COMPLETED / _FAILED are matched against the install/download
//! task ids stored in the node's setup status. (The Go version filtered on a
//! `plugin:files:` task-type prefix that no real event ever carries — the
//! panel sends the domain type, `cmdexec` — so its handler was dead code and
//! installs finished only via the status-poll timeout fallback.)

use gameap_plugin_sdk::proto::gameap::plugin as pb;

use crate::domain::{NodeSetupStatus, SetupStatus};
use crate::host_api::HostApi;
use crate::services::{node_setup, store, sync, users};

pub fn handle<H: HostApi>(host: &mut H, event: &pb::Event) -> pb::EventResult {
    let handled = match event.r#type() {
        pb::EventType::ServerDeleted => handle_server_deleted(host, event),
        pb::EventType::DaemonTaskCompleted => handle_task_event(host, event, true),
        pb::EventType::DaemonTaskFailed => handle_task_event(host, event, false),
        _ => false,
    };
    pb::EventResult {
        handled,
        ..Default::default()
    }
}

fn handle_server_deleted<H: HostApi>(host: &mut H, event: &pb::Event) -> bool {
    let Some(pb::event::Payload::ServerEvent(server_event)) = &event.payload else {
        return false;
    };
    let Some(server) = &server_event.server else {
        return false;
    };
    let server_id = server.id;
    host.log_info(&format!("handling server deletion: server_id={server_id}"));

    let usernames = match users::delete_all_by_server(host, server_id) {
        Ok(usernames) => usernames,
        Err(err) => {
            host.log_error(&format!(
                "failed to delete users from storage: server_id={server_id} error={}",
                err.message
            ));
            Vec::new()
        }
    };

    if !usernames.is_empty() {
        // The payload carries the node id; get_server is only a fallback for
        // events that somehow lack it.
        let node_id = if server.ds_id != 0 {
            Some(server.ds_id)
        } else {
            match sync::node_id_for_server(host, server_id) {
                Ok(id) => Some(id),
                Err(err) => {
                    host.log_error(&format!(
                        "failed to resolve node for deleted server {server_id}: {}",
                        err.message
                    ));
                    None
                }
            }
        };
        if let Some(node_id) = node_id {
            sync::delete_all_users_from_node(host, node_id, &usernames);
        }
    }

    host.log_info(&format!(
        "deleted FTP users for server: server_id={server_id} count={}",
        usernames.len()
    ));
    true
}

fn handle_task_event<H: HostApi>(host: &mut H, event: &pb::Event, completed: bool) -> bool {
    let Some(pb::event::Payload::TaskEvent(task_event)) = &event.payload else {
        return false;
    };
    // Install tasks are created as CMD_EXEC; skip everything else before
    // touching storage (these events fire for every daemon task in the panel).
    if task_event.task_type != "cmdexec" {
        return false;
    }

    let node_id = task_event.node_id;
    let task_id = task_event.task_id;

    let status = match store::get_status(host, node_id) {
        Ok(Some(status)) => status,
        Ok(None) => return false,
        Err(err) => {
            host.log_error(&format!(
                "failed to load setup status: node_id={node_id} error={}",
                err.message
            ));
            return false;
        }
    };
    if status.status != SetupStatus::Installing {
        return false;
    }
    let is_install_task = task_id == status.task_id && status.task_id != 0;
    let is_download_task = task_id == status.download_task_id && status.download_task_id != 0;
    if !is_install_task && !is_download_task {
        return false;
    }

    host.log_info(&format!(
        "handling task completion: task_id={task_id} node_id={node_id} success={completed}"
    ));

    if completed {
        if is_download_task {
            // The install task is still ahead; nothing to record yet.
            return true;
        }
        let new_status = match node_setup::check_installation(host, node_id) {
            Ok(status) => status,
            Err(err) => {
                // Go fell back to "installed" when verification failed.
                host.log_warn(&format!("failed to verify installation: {}", err.message));
                let now = host.now_unix();
                NodeSetupStatus {
                    status: SetupStatus::Installed,
                    task_id,
                    last_check: now,
                    ..NodeSetupStatus::new(SetupStatus::Installed)
                }
            }
        };
        // Persists the outcome and, on success, pushes every user of the node
        // to the (possibly relocated) users directory.
        node_setup::complete_installation(host, node_id, new_status);
        return true;
    }

    // Task failed: recover the failure output from the task itself — the
    // event's extra_data is always empty (which is why the Go plugin's
    // error_message never carried anything).
    let output = match host.find_daemon_task(task_id) {
        Ok(Some(task)) if !task.output.is_empty() => task.output,
        Ok(_) => "installation task failed".into(),
        Err(err) => {
            host.log_warn(&format!(
                "failed to fetch failed task output: {}",
                err.message()
            ));
            "installation task failed".into()
        }
    };
    let now = host.now_unix();
    let new_status = NodeSetupStatus {
        status: SetupStatus::Failed,
        task_id: status.task_id,
        error_message: output,
        last_check: now,
        ..NodeSetupStatus::new(SetupStatus::Failed)
    };
    if let Err(err) = store::save_status(host, node_id, &new_status) {
        host.log_error(&format!("failed to handle task failure: {}", err.message));
    }
    true
}
