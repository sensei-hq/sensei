#!/usr/bin/env bash
# Relay P4.1 -- RLS + realtime-publication proof harness (LOCAL supabase only).
#
# Proves, against the live LOCAL supabase Postgres, that the P4.1 own-rows-only
# model holds for the client-direct Supabase Realtime read path:
#
#   1. member-sees-own   -- a user, reading as their OWN auth.uid() (role
#      `authenticated` + request.jwt.claims, the standard supabase RLS test
#      technique), SELECTs their own session / inbox / segment rows and GETS them.
#   2. non-member-none   -- a DIFFERENT user's auth.uid() gets ZERO rows for those
#      same rows.
#   3. service-role-all  -- the service_role (the Worker's path) still reads AND
#      writes all rows -- RLS bypassed (rolbypassrls).
#   4. in-publication    -- the three relay tables are in `supabase_realtime`.
#
# Seeds minimal fixtures (a tenant + membership + session + segment + inbox for
# two distinct user uuids) inside a transaction, asserts, and ROLLS BACK -- so it
# is repeatable and leaves no residue. LOCAL ONLY -- never run against prod.
#
# Run from the repo root (sensei/). Requires psql. Uses the standard local
# supabase Postgres creds unless DBURL is overridden.
set -uo pipefail
cd "$(dirname "$0")/.."

DBURL=${DBURL:-postgres://postgres:postgres@127.0.0.1:54322/postgres}
PSQL=(psql "$DBURL" -X -q -v ON_ERROR_STOP=1 -tA)

pass=0; fail=0
ok()   { echo "PASS: $*"; pass=$((pass+1)); }
bad()  { echo "FAIL: $*"; fail=$((fail+1)); }
# assert <expected> <actual> <label>
assert() { if [ "$1" = "$2" ]; then ok "$3 (=$2)"; else bad "$3 -- expected '$1' got '$2'"; fi; }

echo "== relay RLS check against $DBURL =="

# --- 4. publication membership (independent of the fixture) ------------------
PUBCOUNT=$("${PSQL[@]}" -c "
  select count(*) from pg_publication_tables
  where pubname='supabase_realtime' and schemaname='dojo'
    and tablename in ('relay_sessions','relay_segments','relay_inbox');") || bad "publication query failed"
assert 3 "$PUBCOUNT" "in-publication: relay_{sessions,segments,inbox} in supabase_realtime"

# --- RLS enabled at all? (guards against a silent 'RLS off' false-positive) --
RLSCOUNT=$("${PSQL[@]}" -c "
  select count(*) from pg_class c join pg_namespace n on n.oid=c.relnamespace
  where n.nspname='dojo' and c.relrowsecurity
    and c.relname in ('relay_sessions','relay_segments','relay_inbox');")
assert 3 "$RLSCOUNT" "rls-enabled: row level security on all three relay tables"

# --- Seed fixtures + run all RLS assertions inside ONE transaction, rollback --
# Everything below runs server-side so `set local role` / `set_config(...,true)`
# (transaction-local) applies to the same session and is auto-reverted on
# ROLLBACK. Each assertion prints "key=<value>"; we parse below.
OUT=$("${PSQL[@]}" <<'SQL'
begin;

-- minimal fixture graph. Fixture uuids are all valid 8-4-4-4-12 hex.
-- user A (owner) = 1111...  user B (other) = 2222...
insert into dojo.tenants (id, key, origin, org, name, dojo_url)
values ('abababab-0000-0000-0000-000000000001',
        'rlscheck/tenant','org','rlscheck','RLS check tenant','http://local.test');

insert into dojo.memberships (id, tenant_id, user_id, dojo_url, kind, authenticated_via)
values ('abababab-0000-0000-0000-0000000000a1',
        'abababab-0000-0000-0000-000000000001',
        '11111111-1111-1111-1111-111111111111',
        'http://local.test','personal','device_code');

insert into dojo.relay_sessions (id, tenant_id, membership_id, user_id, run_id, title)
values ('abababab-0000-0000-0000-000000005551',
        'abababab-0000-0000-0000-000000000001',
        'abababab-0000-0000-0000-0000000000a1',
        '11111111-1111-1111-1111-111111111111',
        'abababab-0000-0000-0000-000000007771',
        'A run');

insert into dojo.relay_segments (id, session_id, seq, title)
values ('abababab-0000-0000-0000-000000006661',
        'abababab-0000-0000-0000-000000005551', 1, 'A segment');

insert into dojo.relay_inbox (id, session_id, segment_id, tenant_id, membership_id,
                             user_id, kind, direction)
values ('abababab-0000-0000-0000-000000008881',
        'abababab-0000-0000-0000-000000005551',
        'abababab-0000-0000-0000-000000006661',
        'abababab-0000-0000-0000-000000000001',
        'abababab-0000-0000-0000-0000000000a1',
        '11111111-1111-1111-1111-111111111111',
        'approval','agent_to_human');

-- 3. service-role: reads all AND can write (RLS bypassed)
set local role service_role;
select 'svc_sessions=' || count(*) from dojo.relay_sessions
  where id='abababab-0000-0000-0000-000000005551';
select 'svc_inbox='    || count(*) from dojo.relay_inbox
  where id='abababab-0000-0000-0000-000000008881';
select 'svc_segments=' || count(*) from dojo.relay_segments
  where id='abababab-0000-0000-0000-000000006661';
update dojo.relay_inbox set status='answered', answered_at=now()
  where id='abababab-0000-0000-0000-000000008881';
select 'svc_write=' || count(*) from dojo.relay_inbox
  where id='abababab-0000-0000-0000-000000008881' and status='answered';
reset role;

-- 1. member-sees-own: user A reading as auth.uid()=A gets their rows
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"11111111-1111-1111-1111-111111111111","role":"authenticated"}', true);
select 'own_sessions=' || count(*) from dojo.relay_sessions
  where id='abababab-0000-0000-0000-000000005551';
select 'own_inbox='    || count(*) from dojo.relay_inbox
  where id='abababab-0000-0000-0000-000000008881';
select 'own_segments=' || count(*) from dojo.relay_segments
  where id='abababab-0000-0000-0000-000000006661';
reset role;

-- 2. non-member-none: user B reading as auth.uid()=B sees ZERO of A rows
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"22222222-2222-2222-2222-222222222222","role":"authenticated"}', true);
select 'other_sessions=' || count(*) from dojo.relay_sessions
  where id='abababab-0000-0000-0000-000000005551';
select 'other_inbox='    || count(*) from dojo.relay_inbox
  where id='abababab-0000-0000-0000-000000008881';
select 'other_segments=' || count(*) from dojo.relay_segments
  where id='abababab-0000-0000-0000-000000006661';
reset role;

rollback;
SQL
) || { echo "FAIL: fixture/RLS transaction errored:"; echo "$OUT"; exit 1; }

# parse "key=value" lines out of the transaction output
val() { echo "$OUT" | grep -m1 "^$1=" | cut -d= -f2; }

# 3. service-role
assert 1 "$(val svc_sessions)" "service-role: reads A session"
assert 1 "$(val svc_inbox)"    "service-role: reads A inbox"
assert 1 "$(val svc_segments)" "service-role: reads A segment"
assert 1 "$(val svc_write)"    "service-role: WRITE (answer inbox) succeeds under RLS"

# 1. member-sees-own
assert 1 "$(val own_sessions)" "member-sees-own: A sees A session"
assert 1 "$(val own_inbox)"    "member-sees-own: A sees A inbox"
assert 1 "$(val own_segments)" "member-sees-own: A sees A segment"

# 2. non-member-none
assert 0 "$(val other_sessions)" "non-member-none: B sees 0 of A sessions"
assert 0 "$(val other_inbox)"    "non-member-none: B sees 0 of A inbox"
assert 0 "$(val other_segments)" "non-member-none: B sees 0 of A segments"

echo "== $pass passed, $fail failed =="
[ "$fail" -eq 0 ] || exit 1
echo "ALL RLS + publication assertions passed."
