#!/usr/bin/env bash
# Headless two-account hub walkthrough: register, friend, find/join a match.
set -euo pipefail

HUB_URL="${STRIKE_HUB_URL:-http://127.0.0.1:7777}"
HOST_USER="${CSRS_USER:-kevin}"
FRIEND_USER="${CSRS_FRIEND:-cedar}"
PASS="${CSRS_PASS:-open-strike-demo}"
MODE="${CSRS_ONLINE_MODE:-tdm}"

json() {
    python3 -c 'import json,sys; print(json.load(sys.stdin)'"$1"')'
}

req() {
    local method="$1"
    local path="$2"
    local token="${3:-}"
    local body="${4:-}"
    local args=(-sS -X "$method" "${HUB_URL}${path}" -H "Content-Type: application/json")
    if [ -n "$token" ]; then
        args+=(-H "Authorization: Bearer ${token}")
    fi
    if [ -n "$body" ]; then
        args+=(-d "$body")
    fi
    curl "${args[@]}"
}

if ! curl -fsS "${HUB_URL}/v1/health" >/dev/null; then
    echo "hub is not reachable at ${HUB_URL}" >&2
    exit 1
fi

auth() {
    local user="$1"
    local email="$2"
    local raw
    raw="$(req POST /v1/auth/register "" "{\"username\":\"${user}\",\"email\":\"${email}\",\"password\":\"${PASS}\"}")" || true
    if echo "$raw" | grep -q '"session"'; then
        echo "$raw"
        return
    fi
    req POST /v1/auth/login "" "{\"identifier\":\"${user}\",\"password\":\"${PASS}\"}"
}

HOST_AUTH="$(auth "${HOST_USER}" "${HOST_USER}@example.com")"
FRIEND_AUTH="$(auth "${FRIEND_USER}" "${FRIEND_USER}@example.com")"
HOST_TOKEN="$(printf '%s' "$HOST_AUTH" | json '["session"]')"
FRIEND_TOKEN="$(printf '%s' "$FRIEND_AUTH" | json '["session"]')"

req POST /v1/friends/request "${HOST_TOKEN}" "{\"username\":\"${FRIEND_USER}\"}" >/dev/null
req POST /v1/friends/accept "${FRIEND_TOKEN}" "{\"username\":\"${HOST_USER}\"}" >/dev/null
FRIENDS="$(req GET /v1/friends "${HOST_TOKEN}")"
echo "$FRIENDS" | python3 -c "
import json,sys
data=json.load(sys.stdin)
names=[f['username'] for f in data.get('friends',[])]
assert '${FRIEND_USER}' in names, data
print('friends ok:', ', '.join(names))
"

MATCH=""
i=0
while [ "$i" -lt 60 ]; do
    MATCH="$(req POST /v1/match/find "${HOST_TOKEN}" "{\"mode\":\"${MODE}\",\"map\":\"dust2\"}")"
    if echo "$MATCH" | grep -q '"ticket"'; then
        break
    fi
    i=$((i + 1))
    sleep 1
done
echo "$MATCH" | python3 -c "
import json,sys
data=json.load(sys.stdin)
server=data.get('server') or {}
assert data.get('ticket'), data
assert server.get('port'), data
print('host ticket ok: {id} {host}:{port} {mode}/{map}'.format(
    id=server.get('server_id'),
    host=server.get('host'),
    port=server.get('port'),
    mode=server.get('mode'),
    map=server.get('map'),
))
"

SERVER_ID="$(printf '%s' "$MATCH" | json '["server"]["server_id"]')"
JOIN="$(req POST /v1/match/join "${FRIEND_TOKEN}" "{\"server_id\":\"${SERVER_ID}\"}")"
echo "$JOIN" | python3 -c "
import json,sys
data=json.load(sys.stdin)
assert data.get('ticket'), data
print('friend join ok:', data['server']['server_id'])
"

echo "hub pair passed against ${HUB_URL}"
