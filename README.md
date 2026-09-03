# GameAP Files Plugin

FTP/SFTP management plugin for the [GameAP](https://gameap.com) control panel.
Installs and manages the [gameap-files](https://github.com/gameap/gameap-files)
FTP/SFTP daemon on nodes and manages per-server FTP users, access rules,
virtual path mounts and SSH keys.

Rust rewrite of the original Go plugin (`plugin-gameap-files`). Storage data,
node-side YAML files and the HTTP API are fully compatible — existing installs
keep working after the swap.

*Читайте на других языках: [Русский](README_RU.md)*

## Features

- One-click gameap-files installation on Linux and Windows nodes (chained
  daemon tasks, live status tracking through daemon-task events with a
  poll/timeout fallback) and an **Update** button that re-runs the installer
  with the stored settings to upgrade an installed node
- Per-node FTP/SFTP configuration (`config.yaml` is patched in place — keys
  the plugin does not own are preserved); the service is restarted through
  the system unit, the user unit of a rootless daemon, or the Windows service
- FTP/SFTP users per game server: create/update/delete, Argon2id password
  hashing via the panel's crypto host service, one-time generated passwords
- Path access rules (`read` / `write` / `delete` / `list`), virtual path
  mounts, SSH public keys
- Users are mirrored to nodes as hot-reloaded YAML drop-ins under
  `<work_path>/.plugins/files/users.d/` — the plugin's service directory
  inside the daemon work path, the one place the daemon lets a panel plugin
  write to (relative home directories are anchored there as well)
- Admin pages: all nodes with install status, all users grouped by
  node → server with filters
- Server abilities `ftp-users-view` / `ftp-users-manage` for non-admin access
  control

## Architecture

| Layer | Module | Responsibility |
|---|---|---|
| ABI | `src/lib.rs` | `Plugin` impl, `register_plugin!`, embedded frontend |
| Transport | `src/http.rs`, `src/router.rs` | JSON error model, route table, dispatch |
| Handlers | `src/handlers/*` | Per-route logic and DTOs, event handling |
| Services | `src/services/*` | Users, sync, node setup orchestration, YAML patching, admin aggregation |
| Domain | `src/domain/*` | Wire-compatible model, validation |
| Host seam | `src/host_api.rs` | `HostApi` trait + `WasmHost` (wasm) / `MockHost` (tests) |

Business logic never touches the host ABI directly — everything goes through
the `HostApi` trait, so the whole router + handler stack runs natively under
`cargo test` against an in-memory mock host.

### Data storage

Panel KV storage (compatible with the Go plugin):

| Scope | Key | Contents |
|---|---|---|
| node | `ftp:setup_status` | Installation state, task ids, timestamps |
| node | `ftp:node_config` | FTP/SFTP configuration |
| server | `ftp:users_list` | Username index (JSON array) |
| server | `ftp:user:{username}` | Full user document (JSON) |

### Events

- `SERVER_DELETED` — removes the server's users from storage and their YAML
  files from the node (node id taken from the event payload)
- `DAEMON_TASK_COMPLETED` / `DAEMON_TASK_FAILED` — matched against the
  install/download task ids stored in the node's setup status; a completed
  installation re-syncs every user of the node once (`synced_after_install`
  in the stored status) and removes the misplaced
  `<work_path>/etc/gameap-files/users.d/*.yaml` files older releases wrote

### Node layout

gameap-daemon confines a panel plugin's file operations to the node work
path, so everything this plugin writes is addressed relative to it:

| Node path (relative to `work_path`) | Contents |
|---|---|
| `.plugins/files/config.yaml` | gameap-files configuration, patched by the settings dialog |
| `.plugins/files/users.d/<user>.yaml` | one drop-in per FTP user |
| `tools/install-files-linux.sh`, `tools/install-files-windows.ps1` | installers fetched with `get-tool` |
| `tools/gameap-files/` (Windows) | binary and service of the Windows install |

### Upgrading from 0.7.x

Nodes installed by an earlier plugin release keep reading `/etc/gameap-files`
until **Update** is clicked once: the installer migrates that directory into
`<work_path>/.plugins/files`, the plugin re-syncs the node's users and sweeps
the files the old release had left under `<work_path>/etc/gameap-files`.
Until then, **Settings** on such a node fails with "failed to download
config", and users created in the panel are not seen by gameap-files.

## Building

Path dependencies require a sibling checkout layout:

```
gameap-api/      # gameap/gameap — provides web/plugin-sdk for the frontend
gameap-proto/    # gameap/gameap-proto — provides rust/gameap-plugin-sdk
plugin-files/    # this repository
```

Requirements: Rust (pinned by `rust-toolchain.toml`, target `wasm32-wasip1`),
Node.js 22+, optionally [binaryen](https://github.com/WebAssembly/binaryen)
for `wasm-opt`.

```bash
make build   # frontend (npm ci + vite) → cargo build → wasm-opt → files.wasm
make test    # cargo test + frontend vitest
make lint    # clippy (both targets) + vue-tsc
```

Development loop:

```bash
cd frontend && npm run dev     # rebuild frontend on change
cargo build --target wasm32-wasip1 --release   # rebuild wasm
cd frontend && npm run debug   # standalone UI against MSW mocks
```

## Installation

Upload `files.wasm` via **Administration → Plugins** or drop it into the
panel's plugins directory and restart GameAP.

## API

All routes live under `/api/plugins/files`. See
[`openapi/openapi.yaml`](openapi/openapi.yaml) for the full specification:
node setup/status/config, user CRUD, access rules, virtual paths, SSH keys
and the admin endpoints.

## Releasing

1. Bump `version` in `Cargo.toml` **and** `frontend/package.json` (must match).
2. Merge, create a GitHub release with tag `v<version>`.
3. The release workflow builds, GPG-signs and publishes the wasm to
   plugins.gameap.dev (requires the `GPG_SIGNING_KEY` / `GAMEAP_DEPLOY_TOKEN`
   secrets and the `GAMEAP_PLUGIN_ID` repository variable).

## License

MIT
