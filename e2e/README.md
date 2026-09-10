# End-to-end suite

Drives a real GameAP panel and a real node through the whole flow this plugin
exists for: upload `files.wasm`, install `gameap-files` on the node from the
admin page, create an FTP user on a game server, log in over FTP and SFTP as
that user, change the node settings, delete the user.

Everything that can be observed only on the node — the systemd unit or the
Windows service, `config.yaml`, the `users.d` drop-in — is checked through a
node abstraction, so one spec suite covers both operating systems.

## Layout

| Path             | Contents                                                          |
|------------------|-------------------------------------------------------------------|
| `specs/`         | The scenario, in file order; each file depends on the ones before  |
| `fixtures/`      | Panel and plugin REST clients, UI helpers, FTP/SFTP clients        |
| `fixtures/node/` | The Linux (docker) and Windows (local) node implementations        |
| `scripts/`       | Panel + node provisioning and log collection, one pair per OS      |

The suite mutates a live system in a fixed order, so `playwright.config.ts`
sets one worker, no parallelism and no retries. A retry would replay a mutation
against an already-mutated system.

## Environment

| Variable               | Meaning                                                  |
|------------------------|-----------------------------------------------------------|
| `E2E_BASE_URL`         | Panel URL the browser opens (default `http://127.0.0.1:8025`) |
| `E2E_API_BASE_URL`     | Panel URL the API calls use                               |
| `E2E_ADMIN_USER`       | Panel admin login (default `admin`)                       |
| `E2E_ADMIN_PASSWORD`   | Panel admin password, required                            |
| `E2E_NODE_OS`          | `linux` or `windows`, picks the node implementation       |
| `E2E_NODE_HOST`        | Address the FTP and SFTP clients connect to               |
| `E2E_NODE_WORK_PATH`   | Daemon work path, `/srv/gameap` or `C:\gameap`            |
| `E2E_NODE_CONTAINER`   | Linux only: the container the node runs in                |
| `E2E_WASM_PATH`        | The `files.wasm` to upload                                |
| `E2E_RUN_ID`           | Salts every created name; defaults to `local`             |
| `E2E_STATE_DIR`        | Where the suite keeps what it has to carry between specs  |

The panel itself has to run with `AUTH_REQUIRE_MFA_FOR_ADMINS=false` (the admin
has no second factor) and `PLUGINS_PERMISSIONS_ENFORCE=true` (otherwise the
permission assertions are decorative). A fixed `DAEMON_SETUP_KEY` is what lets
a node enroll without an API round trip, and `GRPC_EXTERNAL_HOST` has to be set
before the panel starts for the first time because it feeds the SANs of the
gRPC certificate the daemon later verifies.

## Running it

In CI this is `.github/workflows/e2e.yml`: nightly behind a commit gate,
`workflow_dispatch` with `legs` / `panel_source` / `panel_tag` inputs, and on a
pull request only when it carries the `e2e` label.

Locally, provision the same stack and point the suite at it:

```bash
# Linux: a panel on this machine and a node in a systemd container
export ADMIN_PASSWORD=... DAEMON_SETUP_KEY=... FILES_LOCAL_BASE_PATH=/tmp/gameap/files
export E2E_ADMIN_PASSWORD="${ADMIN_PASSWORD}" E2E_NODE_WORK_PATH=/srv/gameap
./scripts/provision-linux.sh

export E2E_NODE_OS=linux E2E_WASM_PATH=../files.wasm
npm ci && npx playwright install --with-deps chromium
npx playwright test
```

A single spec: `npx playwright test specs/02-node-install.spec.ts`. Note that
the later specs assume the earlier ones have run against the same panel.

## A note on the artifacts

Playwright traces capture request bodies, so a trace from a run against
anything other than a throwaway panel will contain the FTP password the suite
created. The log collectors deliberately copy out the `users.d` listing and
never the files, which hold Argon2id hashes, and never the database.
