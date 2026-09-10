#!/usr/bin/env bash
# Brings up the Linux leg: a real panel on the runner and a real node in a
# systemd-booted container. Exports E2E_NODE_HOST, PANEL_TAG and GAMEAPCTL_TAG
# through $GITHUB_ENV when running under Actions.
#
# The node has to boot systemd because install-files-linux.sh refuses to run
# without systemctl, and the daemon is installed by gameapctl because that is
# how an operator provisions a node.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PANEL_DIR="${PANEL_DIR:-/tmp/gameap-panel}"
PANEL_BINARY="${PANEL_BINARY:-}"
PANEL_TAG="${PANEL_TAG:-}"
GAMEAPCTL_TAG="${GAMEAPCTL_TAG:-}"
NODE_IMAGE="${NODE_IMAGE:-gameap-e2e-node:linux}"
CONTAINER="${E2E_NODE_CONTAINER:-gameap-node}"
NODE_WORK_PATH="${E2E_NODE_WORK_PATH:-/srv/gameap}"
PANEL_LOG="${PANEL_LOG:-/tmp/gameap-panel.log}"
API_URL="${E2E_API_BASE_URL:-http://127.0.0.1:8025}"
MIN_PANEL_VERSION="4.5.0"

log() { printf '\n=== %s\n' "$*"; }

export_env() {
  printf '%s=%s\n' "$1" "$2"
  if [ -n "${GITHUB_ENV:-}" ]; then
    printf '%s=%s\n' "$1" "$2" >> "${GITHUB_ENV}"
  fi
}

latest_tag() {
  gh release view --repo "$1" --json tagName -q .tagName
}

log "Checking the upstream sources the installer will use"
for url in \
  "https://raw.githubusercontent.com/gameap/scripts/master/ftp/gameap-files/install-files-linux.sh" \
  "https://cdn.gameap.com/gameap-files/releases.json" \
  "https://api.github.com/repos/gameap/gameap-files/releases/latest"
do
  if ! curl -fsIL --max-time 15 -o /dev/null "${url}"; then
    echo "::warning::upstream unreachable, the node install may fail: ${url}"
  fi
done

log "Resolving the panel"
if [ -n "${PANEL_BINARY}" ]; then
  mkdir -p "${PANEL_DIR}"
  install -m 0755 "${PANEL_BINARY}" "${PANEL_DIR}/gameap"
  PANEL_TAG="${PANEL_TAG:-main}"
else
  PANEL_TAG="${PANEL_TAG:-$(latest_tag gameap/gameap)}"
  lowest="$(printf '%s\n' "v${MIN_PANEL_VERSION}" "${PANEL_TAG}" | sort -V | head -1)"
  if [ "${lowest}" != "v${MIN_PANEL_VERSION}" ]; then
    echo "::error::the plugin needs panel >= ${MIN_PANEL_VERSION}, resolved ${PANEL_TAG}"
    exit 1
  fi

  rm -rf "${PANEL_DIR}"
  mkdir -p "${PANEL_DIR}"
  gh release download --repo gameap/gameap "${PANEL_TAG}" \
    --dir "${PANEL_DIR}" --pattern 'gameap-*-linux-amd64.tar.gz*'
  ( cd "${PANEL_DIR}" && tar -xzf gameap-*-linux-amd64.tar.gz )
  chmod 0755 "${PANEL_DIR}/gameap"
fi
export_env PANEL_TAG "${PANEL_TAG}"

log "Starting the panel (${PANEL_TAG})"
mkdir -p "${FILES_LOCAL_BASE_PATH:?FILES_LOCAL_BASE_PATH must be set}"
db_path="${DATABASE_URL#file:}"
mkdir -p "$(dirname "${db_path%%\?*}")"
"${PANEL_DIR}/gameap" > "${PANEL_LOG}" 2>&1 &
echo $! > /tmp/gameap-panel.pid

for _ in $(seq 1 90); do
  if curl -fsS --max-time 5 "${API_URL}/api/health" > /dev/null 2>&1; then
    break
  fi
  sleep 1
done
if ! curl -fsS --max-time 5 "${API_URL}/api/health" > /dev/null 2>&1; then
  echo "::error::the panel never became healthy"
  tail -n 200 "${PANEL_LOG}" || true
  exit 1
fi

log "Building and starting the node container"
docker build --pull -t "${NODE_IMAGE}" "${REPO_ROOT}/.github/e2e/node-linux"
docker rm -f "${CONTAINER}" > /dev/null 2>&1 || true
# --privileged is needed beyond systemd itself: the units gameapctl installs use
# the sandboxing directives that need mount namespaces.
docker run -d --name "${CONTAINER}" \
  --privileged \
  --cgroupns=host \
  -v /sys/fs/cgroup:/sys/fs/cgroup:rw \
  --tmpfs /run \
  --tmpfs /run/lock \
  --add-host=host.docker.internal:host-gateway \
  "${NODE_IMAGE}"

for _ in $(seq 1 60); do
  state="$(docker exec "${CONTAINER}" systemctl is-system-running 2>/dev/null || true)"
  case "${state}" in
    running|degraded)
      echo "systemd state: ${state}"
      break
      ;;
  esac
  if [ "$(docker inspect -f '{{.State.Running}}' "${CONTAINER}")" != "true" ]; then
    echo "::error::the node container exited before systemd came up"
    docker logs "${CONTAINER}" || true
    exit 1
  fi
  sleep 2
done

NODE_IP="$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "${CONTAINER}")"
if [ -z "${NODE_IP}" ]; then
  echo "::error::could not determine the node container IP"
  exit 1
fi
export_env E2E_NODE_HOST "${NODE_IP}"

log "Installing the daemon with gameapctl"
GAMEAPCTL_TAG="${GAMEAPCTL_TAG:-$(latest_tag gameap/gameapctl)}"
export_env GAMEAPCTL_TAG "${GAMEAPCTL_TAG}"

rm -rf /tmp/gameapctl && mkdir -p /tmp/gameapctl
gh release download --repo gameap/gameapctl "${GAMEAPCTL_TAG}" \
  --dir /tmp/gameapctl --pattern 'gameapctl-*-linux-amd64.tar.gz'
tar -xzf /tmp/gameapctl/gameapctl-*-linux-amd64.tar.gz -C /tmp/gameapctl
docker cp /tmp/gameapctl/gameapctl "${CONTAINER}:/usr/local/bin/gameapctl"
docker exec "${CONTAINER}" chmod 0755 /usr/local/bin/gameapctl
docker exec "${CONTAINER}" /usr/local/bin/gameapctl version

docker exec \
  -e HOME=/root \
  -e DEBIAN_FRONTEND=noninteractive \
  -e NEEDRESTART_SUSPEND=1 \
  "${CONTAINER}" \
  /usr/local/bin/gameapctl --non-interactive daemon install \
    --connect "grpc://host.docker.internal:${GRPC_PORT:-31718}/${DAEMON_SETUP_KEY:?DAEMON_SETUP_KEY must be set}" \
    --work-path "${NODE_WORK_PATH}"

docker exec "${CONTAINER}" systemctl is-active gameap-daemon

log "Waiting for the node to come online in the panel"
token="$(curl -fsS --max-time 10 -H 'Content-Type: application/json' \
  -d "$(jq -n --arg l "${E2E_ADMIN_USER:-admin}" --arg p "${E2E_ADMIN_PASSWORD:?}" \
        '{login: $l, password: $p}')" \
  "${API_URL}/api/auth/login" | jq -er .token)"
echo "::add-mask::${token}"

for _ in $(seq 1 60); do
  summary="$(curl -fsS --max-time 5 -H "Authorization: Bearer ${token}" \
    "${API_URL}/api/nodes/summary" || true)"
  if echo "${summary}" | jq -e '(.total >= 1) and (.online >= 1)' > /dev/null 2>&1; then
    break
  fi
  sleep 3
done

nodes="$(curl -fsS --max-time 10 -H "Authorization: Bearer ${token}" "${API_URL}/api/nodes")"
if ! echo "${nodes}" | jq -e 'map(select(.os == "linux" and .enabled)) | length >= 1' > /dev/null; then
  echo "::error::no enabled linux node enrolled: ${nodes}"
  docker exec "${CONTAINER}" journalctl -u gameap-daemon -n 200 --no-pager || true
  tail -n 200 "${PANEL_LOG}" || true
  exit 1
fi

log "Ready: panel ${PANEL_TAG}, node ${NODE_IP} (${NODE_WORK_PATH})"
