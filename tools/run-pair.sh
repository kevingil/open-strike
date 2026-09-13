#!/usr/bin/env bash
# Two clients against one hub: host finds a TDM, friend joins from the sidebar.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

HUB_URL="${STRIKE_HUB_URL:-http://127.0.0.1:7777}"
PAIR_DIR="${STRIKE_PAIR_DIR:-$ROOT/.pair}"
HOST_USER="${CSRS_USER:-kevin}"
FRIEND_USER="${CSRS_FRIEND:-cedar}"
PASS="${CSRS_PASS:-open-strike-demo}"
MODE="${CSRS_ONLINE_MODE:-tdm}"

if [ "${1:-}" = "--api" ]; then
    exec "$ROOT/tools/test-hub-pair.sh"
fi

if ! curl -fsS "${HUB_URL}/v1/health" >/dev/null 2>&1; then
    echo "hub is not reachable at ${HUB_URL}; start it with tools/run-stack.sh --detach" >&2
    exit 1
fi

if [ -z "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
    echo "no display; running hub API pair instead (tools/test-hub-pair.sh)" >&2
    exec "$ROOT/tools/test-hub-pair.sh"
fi

mkdir -p "${PAIR_DIR}/host" "${PAIR_DIR}/friend"
echo "building open-strike"
cargo build --locked --bin open-strike
CLIENT="${ROOT}/target/debug/open-strike"

launch() {
    local role="$1"
    local user="$2"
    local friend="$3"
    local x="$4"
    env \
        STRIKE_HUB_URL="${HUB_URL}" \
        STRIKE_LOCAL_DB="${PAIR_DIR}/${role}/local.db" \
        STRIKE_HUB_CONFIG="${PAIR_DIR}/${role}/hub.json" \
        CSRS_WINDOWED=1 \
        CSRS_WIDTH="${CSRS_WIDTH:-1280}" \
        CSRS_HEIGHT="${CSRS_HEIGHT:-720}" \
        CSRS_WINDOW_X="${x}" \
        CSRS_WINDOW_Y="${CSRS_WINDOW_Y:-40}" \
        CSRS_WINDOW_TITLE="Open Strike (${role})" \
        CSRS_ONLINE_SCENARIO="${role}" \
        CSRS_USER="${user}" \
        CSRS_FRIEND="${friend}" \
        CSRS_PASS="${PASS}" \
        CSRS_EMAIL="${user}@example.com" \
        CSRS_ONLINE_MODE="${MODE}" \
        CSRS_CAPTURE_DIR="${PAIR_DIR}/${role}" \
        "${CLIENT}" &
    echo $! >"${PAIR_DIR}/${role}.pid"
}

launch host "${HOST_USER}" "${FRIEND_USER}" "${CSRS_HOST_WINDOW_X:-40}"
sleep 2
launch friend "${FRIEND_USER}" "${HOST_USER}" "${CSRS_FRIEND_WINDOW_X:-1340}"

echo "pair running against ${HUB_URL} (host=${HOST_USER} friend=${FRIEND_USER})"
echo "PIDs: host $(cat "${PAIR_DIR}/host.pid") friend $(cat "${PAIR_DIR}/friend.pid")"
wait
