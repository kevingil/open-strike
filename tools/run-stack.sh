#!/usr/bin/env bash
# Run hub + match server as one machine (same process tree as the stack image).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DETACH=0
if [ "${1:-}" = "--detach" ]; then
    DETACH=1
    shift
fi

HUB_URL="${STRIKE_HUB_URL:-http://127.0.0.1:7777}"
STACK_DIR="${STRIKE_STACK_DIR:-$ROOT/.stack}"
mkdir -p "$STACK_DIR"

if curl -fsS "${HUB_URL}/v1/health" >/dev/null 2>&1; then
    echo "stack already healthy at ${HUB_URL}"
    exit 0
fi

echo "building strike-hub and strike-server"
cargo build --locked -p strike-hub
cargo build --locked --bin strike-server --no-default-features

HUB_BIN="${STRIKE_HUB_BIN:-$ROOT/target/debug/strike-hub}"
SERVER_BIN="${STRIKE_SERVER_BIN:-$ROOT/target/debug/strike-server}"

export STRIKE_HUB_DB="${STRIKE_HUB_DB:-$STACK_DIR/hub.db}"
export STRIKE_HUB_LISTEN="${STRIKE_HUB_LISTEN:-0.0.0.0:7777}"
export STRIKE_HUB_URL="${HUB_URL}"
export STRIKE_HUB_NAME="${STRIKE_HUB_NAME:-Open Strike Hub}"
export STRIKE_SERVER_KEY="${STRIKE_SERVER_KEY:-dev}"
export STRIKE_SERVER_HOST="${STRIKE_SERVER_HOST:-127.0.0.1}"
export STRIKE_SERVER_BIND="${STRIKE_SERVER_BIND:-0.0.0.0}"
export STRIKE_SERVER_PORT="${STRIKE_SERVER_PORT:-27015}"
export STRIKE_SERVER_MODE="${STRIKE_SERVER_MODE:-tdm}"
export STRIKE_SERVER_MAP="${STRIKE_SERVER_MAP:-dust2}"
export STRIKE_SERVER_BOTS="${STRIKE_SERVER_BOTS:-6}"
export STRIKE_SERVER_MAX_PLAYERS="${STRIKE_SERVER_MAX_PLAYERS:-12}"
export STRIKE_STACK_SERVER="${STRIKE_STACK_SERVER:-1}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp}"

LOG="${STACK_DIR}/stack.log"
PID_FILE="${STACK_DIR}/stack.pid"

run_stack() {
    cd "$ROOT"
    HUB_PID=""
    SERVER_PID=""
    cleanup() {
        if [ -n "${SERVER_PID}" ]; then
            kill "${SERVER_PID}" 2>/dev/null || true
        fi
        if [ -n "${HUB_PID}" ]; then
            kill "${HUB_PID}" 2>/dev/null || true
        fi
        rm -f "$PID_FILE"
    }
    trap cleanup INT TERM EXIT

    "$HUB_BIN" &
    HUB_PID=$!
    echo "$HUB_PID" >"$PID_FILE"

    i=0
    while [ "$i" -lt 60 ]; do
        if curl -fsS "${HUB_URL}/v1/health" >/dev/null 2>&1; then
            break
        fi
        if ! kill -0 "${HUB_PID}" 2>/dev/null; then
            echo "strike-hub exited before becoming healthy" >&2
            exit 1
        fi
        i=$((i + 1))
        sleep 0.5
    done
    if ! curl -fsS "${HUB_URL}/v1/health" >/dev/null 2>&1; then
        echo "strike-hub did not become healthy at ${HUB_URL}" >&2
        exit 1
    fi

    start_server() {
        "$SERVER_BIN" \
            --hub "${HUB_URL}" \
            --port "${STRIKE_SERVER_PORT}" \
            --advertise "${STRIKE_SERVER_HOST}" \
            --mode "${STRIKE_SERVER_MODE}" \
            --map "${STRIKE_SERVER_MAP}" \
            --bots "${STRIKE_SERVER_BOTS}" \
            --max-players "${STRIKE_SERVER_MAX_PLAYERS}" &
        SERVER_PID=$!
        echo "${HUB_PID} ${SERVER_PID}" >"$PID_FILE"
    }

    if [ "${STRIKE_STACK_SERVER}" != "0" ]; then
        start_server
    fi

    echo "stack up: hub ${HUB_URL}  match udp/${STRIKE_SERVER_HOST}:${STRIKE_SERVER_PORT}"
    while true; do
        if ! kill -0 "${HUB_PID}" 2>/dev/null; then
            wait "${HUB_PID}" || true
            exit 1
        fi
        if [ "${STRIKE_STACK_SERVER}" != "0" ]; then
            if [ -z "${SERVER_PID}" ] || ! kill -0 "${SERVER_PID}" 2>/dev/null; then
                echo "strike-server exited; restarting" >&2
                start_server
            fi
        fi
        sleep 1
    done
}

if [ "$DETACH" = "1" ]; then
    # shellcheck disable=SC2094
    run_stack >>"$LOG" 2>&1 &
    disown $! || true
    i=0
    while [ "$i" -lt 60 ]; do
        if curl -fsS "${HUB_URL}/v1/health" >/dev/null 2>&1; then
            echo "stack detached: hub ${HUB_URL} (log ${LOG})"
            exit 0
        fi
        i=$((i + 1))
        sleep 0.5
    done
    echo "stack failed to become healthy; see ${LOG}" >&2
    exit 1
fi

run_stack
