#!/usr/bin/env bash
#
# Self-test for `run.sh --apply`: unmerged DDL applies inside the test's
# transaction and rolls back afterwards. Exit 0 = pass, 1 = fail,
# 69 = database unreachable (same convention as run.sh), 127 = no psql.
#
# Every refusal asserts exit 2 plus its own message fragment, so a bare
# "no such test scope" (the flag missing) fails every case for the
# right reason instead of passing vacuously.
set -uo pipefail

cd "$(dirname "$0")"

FX=fixtures/run-apply
USAGE='usage: run.sh --apply'
failed=0
workdir=""
trap '[ -n "$workdir" ] && rm -rf "$workdir"' EXIT
pass() { echo "self-test-apply: ok: $1"; }
fail() { echo "self-test-apply: FAIL: $1" >&2; failed=$((failed + 1)); }

# expect_refuse <desc> <fragment> <cmd...>: must exit 2 with fragment in output.
expect_refuse() {
    desc="$1"; want="$2"; shift 2
    out=$("$@" 2>&1); code=$?
    if [ "$code" -ne 2 ]; then
        fail "$desc (exit $code, want 2) -- $out"
        return
    fi
    case "$out" in
        *"$want"*) pass "$desc" ;;
        *) fail "$desc (missing: $want) -- $out" ;;
    esac
}

# --- CLI validation (no database needed) ---
expect_refuse "bare --apply shows usage" "$USAGE" ./run.sh --apply
expect_refuse "missing DDL file refused by name" "does-not-exist.ddl" ./run.sh --apply dojo ./does-not-exist.ddl "$FX/ok.probe"
expect_refuse "missing test file refused by name" "does-not-exist.sql" ./run.sh --apply dojo "$FX/a.ddl" ./does-not-exist.sql
expect_refuse "bad plane refused" "mars" ./run.sh --apply mars "$FX/a.ddl" "$FX/ok.probe"
expect_refuse "DDL managing its own transaction refused" "txn.ddl" ./run.sh --apply dojo "$FX/txn.ddl" "$FX/ok.probe"

if ! command -v psql >/dev/null 2>&1; then
    echo "self-test-apply: SKIP — psql not found on PATH" >&2
    exit 127
fi
DOJO_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"
if ! psql "$DOJO_URL" -X -q -c 'select 1' >/dev/null 2>&1; then
    echo "self-test-apply: SKIP — cannot reach $DOJO_URL" >&2
    exit 69
fi

# Catalogue fingerprint of the target database: classes, procs (with body md5
# so a redefined function counts), constraints, namespaces, triggers, policies.
# Temp schemas excluded (backend-local names). Ordered, so equal states give
# equal strings. A concurrent lane DDL mid-run fails loudly; re-run.
fingerprint() {
    psql "$DOJO_URL" -X -q -t -A -c "
select md5(string_agg(x, '|' order by x)) from (
  select 'c:' || n.nspname || '.' || c.relname || ':' || c.relkind::text as x
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and n.nspname not like 'pg\_temp\_%'
  union all
  select 'p:' || n.nspname || '.' || p.proname || ':' || coalesce(md5(p.prosrc), 'bin')
    from pg_proc p join pg_namespace n on n.oid = p.pronamespace
   where n.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and n.nspname not like 'pg\_temp\_%'
  union all
  select 'o:' || ns.nspname || '.' || co.conname
    from pg_constraint co join pg_namespace ns on ns.oid = co.connamespace
   where ns.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and ns.nspname not like 'pg\_temp\_%'
  union all
  select 'n:' || nspname from pg_namespace
   where nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and nspname not like 'pg\_temp\_%'
  union all
  select 't:' || n.nspname || '.' || t.tgname
    from pg_trigger t join pg_class c on c.oid = t.tgrelid
    join pg_namespace n on n.oid = c.relnamespace
   where n.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and n.nspname not like 'pg\_temp\_%'
  union all
  select 'pol:' || schemaname || '.' || tablename || '.' || policyname from pg_policies
   where schemaname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and schemaname not like 'pg\_temp\_%'
) s;"
}

canaries_present() {
    psql "$DOJO_URL" -X -q -t -A -c "select count(*) from pg_proc p join pg_namespace n on n.oid = p.pronamespace where n.nspname = 'dojo' and p.proname in ('__apply_a', '__apply_b');"
}

# --- Core: a.ddl twice (create-or-replace idempotence) + b.ddl + probe, twice.
# Fingerprint before, between and after: nothing may persist.
core() {
    ./run.sh --apply dojo "$FX/a.ddl" "$FX/a.ddl" "$FX/b.ddl" "$FX/ok.probe" 2>&1
}
fp_before=$(fingerprint)
out=$(core); code=$?
fp_mid=$(fingerprint)
out2=$(core); code2=$?
fp_after=$(fingerprint)
if [ "$code" -ne 0 ] || [ "$code2" -ne 0 ]; then
    fail "core case twice (exits $code,$code2, want 0,0) -- $out $out2"
elif [ "$fp_before" != "$fp_mid" ] || [ "$fp_mid" != "$fp_after" ]; then
    fail "catalogue changed across runs ($fp_before / $fp_mid / $fp_after)"
elif [ "$(canaries_present)" != "0" ]; then
    fail "canary functions persisted after the runs"
else
    pass "core case twice green, catalogue identical, no canaries left"
fi

# --- A failing probe fails the run with its own error (kills noop/ignore). ---
out=$(./run.sh --apply dojo "$FX/a.ddl" "$FX/b.ddl" "$FX/failing.probe" 2>&1); code=$?
case "$code:$out" in
    0:*) fail "failing probe exited 0" ;;
    *apply-probe-canary-failure-42*) pass "failing probe fails with its error" ;;
    *) fail "failing probe (exit $code, want the canary message) -- $out" ;;
esac

# --- The probe depends on the DDL (pin, no --apply involved). ---
out=$(psql "$DOJO_URL" -X -q -v ON_ERROR_STOP=1 -c "begin" -f "$FX/ok.probe" -c "rollback" 2>&1); code=$?
if [ "$code" -eq 0 ]; then
    fail "probe without its DDL passed — it proves nothing"
else
    pass "probe without its DDL fails"
fi

# --- Order is given, never re-sorted (b first: its body names missing a). ---
out=$(./run.sh --apply dojo "$FX/b.ddl" "$FX/a.ddl" "$FX/ok.probe" 2>&1); code=$?
case "$code:$out" in
    0:*) fail "reversed DDL order exited 0" ;;
    *__apply_a*) pass "reversed DDL order fails naming the missing dependency" ;;
    *) fail "reversed DDL order (exit $code, want the dependency error) -- $out" ;;
esac

# --- A real-shaped probe (own begin/rollback) runs, canary visible. ---
out=$(./run.sh --apply dojo "$FX/a.ddl" "$FX/b.ddl" "$FX/shaped.probe" 2>&1); code=$?
if [ "$code" -ne 0 ]; then
    fail "real-shaped probe (exit $code, want 0) -- $out"
else
    pass "real-shaped probe green"
fi

# --- Outside the allowed roots (database/ddl, database/tests/fixtures). ---
workdir=$(mktemp -d)
printf 'select 1;\n' > "$workdir/outside.sql"
out=$(./run.sh --apply dojo "$workdir/outside.sql" "$FX/ok.probe" 2>&1); code=$?
case "$code:$out" in
    2*outside.sql*) pass "outside-roots DDL refused by name" ;;
    *) fail "outside-roots DDL (exit $code, want 2 naming the file) -- $out" ;;
esac

# --- From another cwd with repo-relative paths (paths resolve before cd). ---
out=$(cd ../.. && database/tests/run.sh --apply dojo database/tests/fixtures/run-apply/a.ddl database/tests/fixtures/run-apply/b.ddl database/tests/fixtures/run-apply/ok.probe 2>&1); code=$?
if [ "$code" -ne 0 ]; then
    fail "repo-root call (exit $code, want 0) -- $out"
else
    pass "repo-root call with repo-relative paths"
fi

if [ "$failed" -ne 0 ]; then
    echo "self-test-apply: $failed failing case(s)" >&2
    exit 1
fi
echo "self-test-apply: all green"
