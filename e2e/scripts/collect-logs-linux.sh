#!/usr/bin/env bash
# Best-effort diagnostics for a failed Linux leg. Never fails the job, and
# never copies out the database or the raw users.d files: both carry password
# hashes.
set -uo pipefail

OUT="${1:-e2e-logs}"
CONTAINER="${E2E_NODE_CONTAINER:-gameap-node}"
NODE_WORK_PATH="${E2E_NODE_WORK_PATH:-/srv/gameap}"
PANEL_LOG="${PANEL_LOG:-/tmp/gameap-panel.log}"
API_URL="${E2E_API_BASE_URL:-http://127.0.0.1:8025}"

mkdir -p "${OUT}"

cp "${PANEL_LOG}" "${OUT}/panel.log" 2>/dev/null || true
ls -l "${FILES_LOCAL_BASE_PATH:-/tmp/gameap/files}/plugins" > "${OUT}/panel-plugins-dir.txt" 2>&1 || true

login_body="$(jq -n --arg l "${E2E_ADMIN_USER:-admin}" --arg p "${E2E_ADMIN_PASSWORD:-}" \
  '{login: $l, password: $p}')"
if token="$(curl -fsS --max-time 10 -H 'Content-Type: application/json' \
    -d "${login_body}" "${API_URL}/api/auth/login" | jq -er .token 2>/dev/null)"; then
  echo "::add-mask::${token}"
  curl -fsS -H "Authorization: Bearer ${token}" "${API_URL}/api/admin/plugins/loaded" \
    > "${OUT}/plugins-loaded.json" 2>&1 || true
fi

if docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER}"; then
  docker logs "${CONTAINER}" > "${OUT}/node-container.log" 2>&1 || true
  docker exec "${CONTAINER}" journalctl -u gameap-files --no-pager -o short-iso \
    > "${OUT}/journal-gameap-files.log" 2>&1 || true
  docker exec "${CONTAINER}" journalctl -u gameap-daemon --no-pager -o short-iso \
    > "${OUT}/journal-gameap-daemon.log" 2>&1 || true
  docker exec "${CONTAINER}" systemctl --no-pager status gameap-files gameap-daemon \
    > "${OUT}/systemctl-status.txt" 2>&1 || true
  docker exec "${CONTAINER}" cat "${NODE_WORK_PATH}/.plugins/files/config.yaml" \
    > "${OUT}/gameap-files-config.yaml" 2>&1 || true
  # The listing only, never the contents: a drop-in holds an argon2 hash.
  docker exec "${CONTAINER}" ls -l "${NODE_WORK_PATH}/.plugins/files/users.d" \
    > "${OUT}/users-d-listing.txt" 2>&1 || true
  docker exec "${CONTAINER}" ss -lntp > "${OUT}/listening-ports.txt" 2>&1 || true
  docker exec "${CONTAINER}" /usr/local/bin/gameap-files version \
    > "${OUT}/gameap-files-version.txt" 2>&1 || true
  docker exec "${CONTAINER}" sh -c 'cat /var/log/gameapctl/*.log 2>/dev/null' \
    > "${OUT}/gameapctl.log" 2>&1 || true
fi

echo "collected into ${OUT}:"
ls -l "${OUT}" || true

# Diagnostics must never fail the job.
exit 0
