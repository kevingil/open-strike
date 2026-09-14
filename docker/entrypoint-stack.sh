#!/bin/sh
# One-machine hub + match server. Used by the stack image, Fly, and local scripts.
set -eu

if [ -z "${VK_ICD_FILENAMES:-}" ] && [ -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then
    export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
fi
export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
export WGPU_POWER_PREF="${WGPU_POWER_PREF:-low}"

HUB_URL="${STRIKE_HUB_URL:-http://127.0.0.1:7777}"
ADVERTISE="${STRIKE_SERVER_HOST:-127.0.0.1}"
PORT="${STRIKE_SERVER_PORT:-27015}"
MODE="${STRIKE_SERVER_MODE:-tdm}"
MAP="${STRIKE_SERVER_MAP:-dust2}"
BOTS="${STRIKE_SERVER_BOTS:-6}"
MAX_PLAYERS="${STRIKE_SERVER_MAX_PLAYERS:-12}"
RUN_SERVER="${STRIKE_STACK_SERVER:-1}"

HUB_PID=""
SERVER_PID=""

cleanup() {
    if [ -n "${SERVER_PID}" ]; then
        kill "${SERVER_PID}" 2>/dev/null || true
    fi
    if [ -n "${HUB_PID}" ]; then
        kill "${HUB_PID}" 2>/dev/null || true
    fi
}
trap cleanup INT TERM

./strike-hub &
HUB_PID=$!

i=0
while [ "$i" -lt 60 ]; do
    if curl -fsS "${HUB_URL}/v1/health" >/dev/null 2>&1; then
        break
    fi
    if ! kill -0 "${HUB_PID}" 2>/dev/null; then
        echo "strike-hub exited before becoming healthy" >&2
        wait "${HUB_PID}" || true
        exit 1
    fi
    i=$((i + 1))
    sleep 0.5
done

if ! curl -fsS "${HUB_URL}/v1/health" >/dev/null 2>&1; then
    echo "strike-hub did not become healthy at ${HUB_URL}" >&2
    cleanup
    exit 1
fi

start_server() {
    ./strike-server \
        --hub "${HUB_URL}" \
        --port "${PORT}" \
        --advertise "${ADVERTISE}" \
        --mode "${MODE}" \
        --map "${MAP}" \
        --bots "${BOTS}" \
        --max-players "${MAX_PLAYERS}" &
    SERVER_PID=$!
}

if [ "${RUN_SERVER}" != "0" ]; then
    start_server
fi

while true; do
    if ! kill -0 "${HUB_PID}" 2>/dev/null; then
        wait "${HUB_PID}" || status=$?
        cleanup
        exit "${status:-1}"
    fi
    if [ "${RUN_SERVER}" != "0" ]; then
        if [ -z "${SERVER_PID}" ] || ! kill -0 "${SERVER_PID}" 2>/dev/null; then
            echo "strike-server exited; restarting in 2s" >&2
            sleep 2
            start_server
        fi
    fi
    sleep 1
done
