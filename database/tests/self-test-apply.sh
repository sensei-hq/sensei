#!/usr/bin/env bash
#
# Self-test for `run.sh --apply`: proves unmerged DDL applies inside the
# test's transaction and rolls back afterwards. Exit 0 = pass, 1 = fail,
# 69 = database unreachable (same convention as run.sh), 127 = no psql.
#
# Phase A of the run.sh --apply item: fails until --apply exists (run.sh
# answers "no such test scope"). Nothing wires this script yet; the handoff
# asks L0 to run it where the dōjō database exists.
set -uo pipefail

cd "$(dirname "$0")"

fail() { echo "self-test-apply: FAIL: $1" >&2; exit 1; }
pass() { echo "self-test-apply: ok: $1"; }

# --- CLI validation (no database needed) ---
./run.sh --apply >/dev/null 2>&1
[ $? -eq 0 ] && fail "--apply with no args must exit nonzero"
pass "no args rejected"

./run.sh --apply ./does-not-exist.ddl ./dojo/workspaces_read.sql >/dev/null 2>&1
[ $? -eq 0 ] && fail "missing DDL file must exit nonzero"
pass "missing DDL file rejected"

./run.sh --apply ../ddl/function/dojo/is_iso_instant.ddl ./does-not-exist.sql >/dev/null 2>&1
[ $? -eq 0 ] && fail "missing test file must exit nonzero"
pass "missing test file rejected"

if ! command -v psql >/dev/null 2>&1; then
    echo "self-test-apply: SKIP — psql not found on PATH" >&2
    exit 127
fi
DOJO_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"
if ! psql "$DOJO_URL" -X -q -c 'select 1' >/dev/null 2>&1; then
    echo "self-test-apply: SKIP — cannot reach $DOJO_URL" >&2
    exit 69
fi

# --- Core: a canary DDL applies (twice: create-or-replace is idempotent),
# the probe test sees it, and nothing persists afterwards. ---
workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT
cat > "$workdir/canary.ddl" <<'EOF'
create or replace function dojo.__apply_selftest_canary() returns integer
    language sql immutable as $$ select 7 $$;
EOF
cat > "$workdir/probe.sql" <<'EOF'
-- No transaction statements of its own: --apply owns the transaction.
do $$
declare
    v integer;
begin
    select dojo.__apply_selftest_canary() into v;
    if v <> 7 then
        raise exception 'canary function not visible inside the applied transaction, got %', v;
    end if;
end $$;
EOF

if ! ./run.sh --apply "$workdir/canary.ddl" "$workdir/canary.ddl" "$workdir/probe.sql" >/dev/null 2>&1; then
    fail "canary DDL applied twice plus probe test must exit 0"
fi
pass "canary applied twice, probe saw it"

leftovers=$(psql "$DOJO_URL" -X -q -t -c "select count(*) from pg_proc p join pg_namespace n on n.oid = p.pronamespace where n.nspname = 'dojo' and p.proname = '__apply_selftest_canary';" | tr -d ' ')
if [ "$leftovers" != "0" ]; then
    fail "canary function persisted after the run (count=$leftovers), rollback broken"
fi
pass "nothing persisted after the run"

echo "self-test-apply: all green"
