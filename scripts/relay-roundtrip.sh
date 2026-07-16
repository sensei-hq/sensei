#!/usr/bin/env bash
# P1 relay round-trip harness — proves the daemon↔Worker↔phone loop end-to-end
# against LOCAL supabase + a running dojo Worker (default :5173).
#
# Seeds a personal-Dōjō tenant + membership + Supabase auth user (LOCAL ONLY,
# D4 tracked-temporary — never run against prod), then curls the full loop:
#   daemon POST session → daemon POST inbox (gate) → daemon GET inbox?since=0
#   → phone POST reply (JWT) → daemon GET inbox?since=<cursor> re-surfaces the
#   answered row (proving the relay_inbox seq trigger + cursor).
#
# Run from the repo (sensei/). Requires: supabase running, dojo Worker running
# (cd dojo && PUBLIC_SUPABASE_URL=… SUPABASE_SERVICE_ROLE_KEY=… bun run dev), jq.
set -uo pipefail
cd "$(dirname "$0")/.."

WORKER=${WORKER:-http://localhost:5173}
TESTTOKEN=${TESTTOKEN:-relay-dev-device-token}
EMAIL=${EMAIL:-relay-jerry@local.test}
PASS=${PASS:-relaytest123}
fail() { echo "FAIL: $*" >&2; exit 1; }

eval "$(supabase status -o env | sed 's/^/export /')" || fail "supabase status"
API="$API_URL"; DBURL="$DB_URL"

echo "== seed: auth user =="
USER_JSON=$(curl -s -X POST "$API/auth/v1/admin/users" \
  -H "apikey: $SERVICE_ROLE_KEY" -H "Authorization: Bearer $SERVICE_ROLE_KEY" \
  -H 'content-type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASS\",\"email_confirm\":true}")
USER_ID=$(echo "$USER_JSON" | jq -r '.id // empty')
[ -z "$USER_ID" ] && USER_ID=$(psql "$DBURL" -tAc "select id from auth.users where email='$EMAIL' limit 1")
[ -n "$USER_ID" ] || fail "no auth user id ($USER_JSON)"
echo "   user_id=$USER_ID"

HASH=$(printf '%s' "$TESTTOKEN" | shasum -a 256 | cut -d' ' -f1)
echo "== seed: tenant + membership (device_token_hash=$HASH) =="
psql "$DBURL" -v ON_ERROR_STOP=1 <<SQL || fail "seed sql"
insert into dojo.tenants (key, origin, org, name, dojo_url)
values ('personal/jerry','org','jerry','Jerry (personal)','$API')
on conflict (key) do nothing;
insert into dojo.memberships (tenant_id, user_id, dojo_url, role, kind, authenticated_via, device_token_hash)
select t.id, '$USER_ID', '$API', 'admin', 'personal', 'device_code', '$HASH'
from dojo.tenants t where t.key='personal/jerry'
on conflict (tenant_id, user_id) do update set device_token_hash = excluded.device_token_hash;
-- Grant the Supabase API roles access to dojo so PostgREST can read it. PROD
-- Supabase needs the same one-time grant (see docs/plan/decisions.md). Idempotent.
grant usage on schema dojo to service_role, authenticated, anon;
grant all on all tables in schema dojo to service_role;
grant all on all sequences in schema dojo to service_role;
alter default privileges in schema dojo grant all on tables to service_role;
alter default privileges in schema dojo grant all on sequences to service_role;
SQL

echo "== get user JWT (password grant) =="
JWT=$(curl -s -X POST "$API/auth/v1/token?grant_type=password" \
  -H "apikey: $ANON_KEY" -H 'content-type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASS\"}" | jq -r '.access_token // empty')
[ -n "$JWT" ] || fail "no JWT"
echo "   jwt len=${#JWT}"

RUN="$(uuidgen | tr '[:upper:]' '[:lower:]')"   # relay_sessions.run_id is a uuid (the daemon's activity.runs.id)
AUTH_DEV="Authorization: Bearer $TESTTOKEN"
AUTH_JWT="Authorization: Bearer $JWT"
CT='content-type: application/json'

# The vite DEV server compiles a route on first hit and can drop the body on that
# cold request (→ "… is required"). Retry warms it. A real Cloudflare Worker
# compiles ahead of time, so this is a dev-only artifact — not a route bug.
req() { # METHOD URL AUTH [BODY]
  local m=$1 u=$2 a=$3 b=${4:-} r
  for _ in 1 2 3 4 5; do
    if [ -n "$b" ]; then r=$(curl -s -X "$m" "$u" -H "$a" -H "$CT" -d "$b")
    else r=$(curl -s -X "$m" "$u" -H "$a"); fi
    case "$r" in ""|*required*|*"Internal Error"*) sleep 0.4 ;; *) printf '%s' "$r"; return 0 ;; esac
  done
  printf '%s' "$r"
}

# Warm each route once (vite dev compiles a route on first hit and may drop that
# request's body). Results discarded — this just compiles the handlers.
echo "== warm dev routes =="
Z="00000000-0000-0000-0000-000000000000"
curl -s -o /dev/null -X POST "$WORKER/v1/t/personal/jerry/relay/session" -H "$AUTH_DEV" -H "$CT" -d "{\"run_id\":\"$Z\"}" || true
curl -s -o /dev/null -X POST "$WORKER/v1/t/personal/jerry/relay/inbox" -H "$AUTH_DEV" -H "$CT" -d "{\"run_id\":\"$Z\",\"kind\":\"approval\"}" || true
curl -s -o /dev/null "$WORKER/v1/t/personal/jerry/relay/inbox?since=999999999" -H "$AUTH_DEV" || true
curl -s -o /dev/null -X POST "$WORKER/v1/t/personal/jerry/relay/segments" -H "$AUTH_DEV" -H "$CT" -d "{\"run_id\":\"$Z\",\"segments\":[]}" || true
curl -s -o /dev/null "$WORKER/v1/t/personal/jerry/relay/segments?run_id=$Z" -H "$AUTH_JWT" || true
curl -s -o /dev/null -X POST "$WORKER/v1/t/personal/jerry/relay/reply" -H "$AUTH_JWT" -H "$CT" -d "{\"inbox_id\":\"$Z\",\"reply\":{}}" || true
sleep 0.6

echo "== 1. daemon POST session =="
S=$(req POST "$WORKER/v1/t/personal/jerry/relay/session" "$AUTH_DEV" \
  "{\"run_id\":\"$RUN\",\"title\":\"Round-trip\",\"status\":\"running\",\"progress_done\":0,\"progress_total\":1,\"current_phase\":\"P1\"}")
echo "   $S"; echo "$S" | jq -e '.id' >/dev/null || fail "session ($S)"

echo "== 1b. daemon POST segments (outline) =="
SEG=$(req POST "$WORKER/v1/t/personal/jerry/relay/segments" "$AUTH_DEV" \
  "{\"run_id\":\"$RUN\",\"segments\":[{\"seq\":0,\"title\":\"Phase 1\",\"summary\":\"vertical slice\",\"state\":\"active\",\"is_gate\":false},{\"seq\":1,\"title\":\"Gate\",\"state\":\"blocked\",\"is_gate\":true,\"gate_severity\":\"blocking\"}]}")
echo "   $SEG"; echo "$SEG" | jq -e '.upserted==2' >/dev/null || fail "segments publish ($SEG)"

echo "== 1c. phone GET segments =="
GS=$(req GET "$WORKER/v1/t/personal/jerry/relay/segments?run_id=$RUN" "$AUTH_JWT")
echo "   $GS"
echo "$GS" | jq -e '.segments | length==2 and .[0].title=="Phase 1" and .[1].is_gate==true and .[1].gate_severity=="blocking"' >/dev/null || fail "segments read ($GS)"

echo "== 2. daemon POST inbox (raise gate) =="
I=$(req POST "$WORKER/v1/t/personal/jerry/relay/inbox" "$AUTH_DEV" \
  "{\"run_id\":\"$RUN\",\"kind\":\"approval\",\"direction\":\"agent_to_human\",\"payload\":{\"prompt\":\"run cargo test?\"}}")
echo "   $I"; INBOX_ID=$(echo "$I" | jq -r '.id // empty'); [ -n "$INBOX_ID" ] || fail "inbox ($I)"

echo "== 3. daemon GET inbox?since=0 (expect pending) =="
P1=$(req GET "$WORKER/v1/t/personal/jerry/relay/inbox?since=0" "$AUTH_DEV")
echo "   $P1"
echo "$P1" | jq -e '.items[] | select(.id=="'"$INBOX_ID"'" and .status=="pending")' >/dev/null || fail "gate not pending ($P1)"
CURSOR=$(echo "$P1" | jq -r '.cursor')

echo "== 4. phone POST reply (JWT) =="
R=$(req POST "$WORKER/v1/t/personal/jerry/relay/reply" "$AUTH_JWT" \
  "{\"inbox_id\":\"$INBOX_ID\",\"reply\":{\"verdict\":\"approve\"}}")
echo "   $R"; echo "$R" | jq -e '.id' >/dev/null || fail "reply ($R)"

echo "== 5. daemon GET inbox?since=$CURSOR (expect answer re-surfaced) =="
P2=$(req GET "$WORKER/v1/t/personal/jerry/relay/inbox?since=$CURSOR" "$AUTH_DEV")
echo "   $P2"
echo "$P2" | jq -e '.items[] | select(.id=="'"$INBOX_ID"'" and .status=="answered" and .reply.verdict=="approve")' >/dev/null \
  || fail "answer NOT re-surfaced past cursor $CURSOR ($P2)"

echo "ROUND-TRIP OK ✓ — gate raised, answered from the phone plane, re-surfaced to the daemon poll via the seq trigger."
