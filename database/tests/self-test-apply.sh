#!/usr/bin/env bash
#
# Self-test for `run.sh --apply`: unmerged DDL applies inside the test's
# transaction and rolls back afterwards. Exit 0 = pass, 1 = fail,
# 69 = database unreachable (same convention as run.sh), 127 = no psql.
#
# Every refusal asserts exit 2 plus its own message fragment, so a bare
# "no such test scope" (the flag missing) fails every case for the
# right reason instead of passing vacuously. Every behaviour pin pairs a
# positive with a negative, so a probe that never runs cannot pass.
set -uo pipefail

cd "$(dirname "$0")"

FX=fixtures/run-apply
USAGE='usage: run.sh --apply'
ROOTS_MSG='outside the allowed roots'
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

# expect_roots_refuse <desc> <path-fragment> <cmd...>: must exit 2 with the
# roots rule and the path in the output. The rule phrase matters: psql -f
# errors echo the file path, so the path alone would pass a guard that lets
# the file through and fails on its contents instead.
expect_roots_refuse() {
    desc="$1"; want="$2"; shift 2
    out=$("$@" 2>&1); code=$?
    if [ "$code" -ne 2 ]; then
        fail "$desc (exit $code, want 2) -- $out"
        return
    fi
    case "$out" in
        *"$ROOTS_MSG"*) : ;;
        *) fail "$desc (missing: $ROOTS_MSG) -- $out"; return ;;
    esac
    case "$out" in
        *"$want"*) pass "$desc" ;;
        *) fail "$desc (missing path: $want) -- $out" ;;
    esac
}

# --- CLI validation (no database needed) ---
expect_refuse "bare --apply shows usage" "$USAGE" ./run.sh --apply
expect_refuse "missing DDL file refused by name" "does-not-exist.ddl" ./run.sh --apply dojo ./does-not-exist.ddl "$FX/ok.probe"
expect_refuse "missing test file refused by name" "does-not-exist.sql" ./run.sh --apply dojo "$FX/a.ddl" ./does-not-exist.sql
expect_refuse "bad plane refused" "mars" ./run.sh --apply mars "$FX/a.ddl" "$FX/ok.probe"
# The transaction-guard fixtures hold one statement each and create nothing,
# so they persist nothing even when the guard is missing (F3).
expect_refuse "DDL with commit; refused" "txn-commit.ddl" ./run.sh --apply dojo "$FX/txn-commit.ddl" "$FX/ok.probe"
expect_refuse "DDL with ROLLBACK; refused" "txn-rollback.ddl" ./run.sh --apply dojo "$FX/txn-rollback.ddl" "$FX/ok.probe"
expect_refuse "DDL with commit work; refused" "txn-commit-work.ddl" ./run.sh --apply dojo "$FX/txn-commit-work.ddl" "$FX/ok.probe"
# Test files get a commit-only guard (F5): begin;/rollback; stay allowed, and
# noop.ddl persists nothing even if the guard is missing and it commits.
expect_refuse "test file with commit; refused" "commits.probe" ./run.sh --apply dojo "$FX/noop.ddl" "$FX/commits.probe"
expect_refuse "test file with COMMIT WORK; refused" "commits-work.probe" ./run.sh --apply dojo "$FX/noop.ddl" "$FX/commits-work.probe"

# A broken argument check exits 1, never "skipped": 69/127 mean only that the
# database or psql is absent and every CLI case passed (F4).
if ! command -v psql >/dev/null 2>&1; then
    if [ "$failed" -ne 0 ]; then
        echo "self-test-apply: $failed failing case(s)" >&2
        exit 1
    fi
    echo "self-test-apply: SKIP — psql not found on PATH" >&2
    exit 127
fi
DOJO_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"
SENSEI_URL="${SENSEI_DATABASE_URL:-postgresql://localhost:5432/sensei}"
if ! psql "$DOJO_URL" -X -q -c 'select 1' >/dev/null 2>&1; then
    if [ "$failed" -ne 0 ]; then
        echo "self-test-apply: $failed failing case(s)" >&2
        exit 1
    fi
    echo "self-test-apply: SKIP — cannot reach $DOJO_URL" >&2
    exit 69
fi

# Catalogue fingerprint of the target database: classes, columns, procs (with
# body md5 so a redefined function counts), constraints, namespaces, triggers,
# policies. Temp schemas excluded, toast-owned ones too (a concurrent vacuum's
# temp table must not flake the count). Ordered, so equal states give equal
# strings. A concurrent lane DDL mid-run fails loudly; re-run.
fingerprint() {
    psql "$1" -X -q -t -A -c "
select md5(string_agg(x, '|' order by x)) from (
  select 'c:' || n.nspname || '.' || c.relname || ':' || c.relkind::text as x
    from pg_class c join pg_namespace n on n.oid = c.relnamespace
   where n.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and n.nspname not like 'pg\_%temp\_%'
  union all
  select 'a:' || n.nspname || '.' || c.relname || '.' || a.attname || ':' || format_type(a.atttypid, a.atttypmod)
    from pg_attribute a join pg_class c on c.oid = a.attrelid
    join pg_namespace n on n.oid = c.relnamespace
   where a.attnum > 0 and not a.attisdropped
     and n.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and n.nspname not like 'pg\_%temp\_%'
  union all
  select 'p:' || n.nspname || '.' || p.proname || ':' || coalesce(md5(p.prosrc), 'bin')
    from pg_proc p join pg_namespace n on n.oid = p.pronamespace
   where n.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and n.nspname not like 'pg\_%temp\_%'
  union all
  select 'o:' || ns.nspname || '.' || co.conname
    from pg_constraint co join pg_namespace ns on ns.oid = co.connamespace
   where ns.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and ns.nspname not like 'pg\_%temp\_%'
  union all
  select 'n:' || nspname from pg_namespace
   where nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and nspname not like 'pg\_%temp\_%'
  union all
  select 't:' || n.nspname || '.' || t.tgname
    from pg_trigger t join pg_class c on c.oid = t.tgrelid
    join pg_namespace n on n.oid = c.relnamespace
   where n.nspname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and n.nspname not like 'pg\_%temp\_%'
  union all
  select 'pol:' || schemaname || '.' || tablename || '.' || policyname from pg_policies
   where schemaname not in ('pg_catalog', 'information_schema', 'pg_toast')
     and schemaname not like 'pg\_%temp\_%'
) s;"
}

# Every fixture object shares the __apply prefix, in any schema, so one
# pattern covers them all with the readable canary message.
canaries_present() {
    psql "$1" -X -q -t -A -c "select count(*) from pg_proc p where left(p.proname, 7) = '__apply';"
}

# fp_before precedes the first case that can reach the database: the CLI cases
# above must never touch one, and the guard re-runs below prove the txn
# fixtures leave no trace whether the guard exists or not (F3).
fp_before=$(fingerprint "$DOJO_URL")
expect_refuse "DDL with commit; refused (no-trace re-run)" "txn-commit.ddl" ./run.sh --apply dojo "$FX/txn-commit.ddl" "$FX/ok.probe"
expect_refuse "DDL with ROLLBACK; refused (no-trace re-run)" "txn-rollback.ddl" ./run.sh --apply dojo "$FX/txn-rollback.ddl" "$FX/ok.probe"
expect_refuse "DDL with commit work; refused (no-trace re-run)" "txn-commit-work.ddl" ./run.sh --apply dojo "$FX/txn-commit-work.ddl" "$FX/ok.probe"
expect_refuse "test file with commit; refused (no-trace re-run)" "commits.probe" ./run.sh --apply dojo "$FX/noop.ddl" "$FX/commits.probe"
expect_refuse "test file with COMMIT WORK; refused (no-trace re-run)" "commits-work.probe" ./run.sh --apply dojo "$FX/noop.ddl" "$FX/commits-work.probe"
fp_guards=$(fingerprint "$DOJO_URL")
if [ "$fp_guards" != "$fp_before" ]; then
    fail "catalogue changed across the guard cases ($fp_before / $fp_guards)"
elif [ "$(canaries_present "$DOJO_URL")" != "0" ]; then
    fail "canary functions persisted after the guard cases"
else
    pass "guard cases persist nothing"
fi

# --- Core: a.ddl twice (create-or-replace idempotence) + b.ddl + probe, twice.
# Fingerprint before, between and after: nothing may persist.
core() {
    ./run.sh --apply dojo "$FX/a.ddl" "$FX/a.ddl" "$FX/b.ddl" "$FX/ok.probe" 2>&1
}
out=$(core); code=$?
fp_mid=$(fingerprint "$DOJO_URL")
out2=$(core); code2=$?
fp_after=$(fingerprint "$DOJO_URL")
if [ "$code" -ne 0 ] || [ "$code2" -ne 0 ]; then
    fail "core case twice (exits $code,$code2, want 0,0) -- $out $out2"
elif [ "$fp_before" != "$fp_mid" ] || [ "$fp_mid" != "$fp_after" ]; then
    fail "catalogue changed across runs ($fp_before / $fp_mid / $fp_after)"
elif [ "$(canaries_present "$DOJO_URL")" != "0" ]; then
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

# --- F2: the guard lets real DDL through (plpgsql bodies hold whole-line begin/end). ---
out=$(./run.sh --apply dojo ../ddl/function/dojo/is_iso_instant.ddl "$FX/is-iso.probe" 2>&1); code=$?
if [ "$code" -ne 0 ]; then
    fail "real plpgsql DDL applies (exit $code, want 0) -- $out"
else
    pass "real plpgsql DDL applies"
fi

# --- L3: a transaction a DDL file ends mid-line is detected, naming the file.
# rollback (not commit), so nothing persists while detection is missing. ---
out=$(./run.sh --apply dojo "$FX/midline.ddl" "$FX/ok.probe" 2>&1); code=$?
case "$code:$out" in
    0:*) fail "midline transaction-end exited 0" ;;
    *midline.ddl*) pass "midline transaction-end detected and named" ;;
    *) fail "midline transaction-end (exit $code, want nonzero naming the file) -- $out" ;;
esac

# --- L-a: top-level END (= COMMIT) in a test aborts the wrapper. ---
# The deferred commit trap turns the commit into an abort; the refusal names
# itself, and psql names the file that issued the END.
out=$(./run.sh --apply dojo "$FX/noop.ddl" "$FX/end.probe" 2>&1); code=$?
if [ "$code" -eq 0 ]; then
    fail "END-in-test exited 0"
else
    case "$out" in
        *"wrapper transaction was committed"*)
            case "$out" in
                *end.probe*) pass "END-in-test aborts naming the refusal and the file" ;;
                *) fail "END-in-test aborts without naming the file -- $out" ;;
            esac ;;
        *) fail "END-in-test (exit $code, want the commit-trap refusal) -- $out" ;;
    esac
fi

# --- L-b: the marker is checked after EACH DDL file, not once at the end. ---
# after-midline.ddl persists nothing but announces itself; it must never run.
out=$(./run.sh --apply dojo "$FX/midline.ddl" "$FX/after-midline.ddl" "$FX/ok.probe" 2>&1); code=$?
case "$code:$out" in
    0:*) fail "post-midline DDL ran to exit 0" ;;
    *midline.ddl*)
        case "$out" in
            *apply-after-midline-ran*) fail "post-midline DDL ran after the transaction ended" ;;
            *) pass "post-midline DDL never ran, midline named" ;;
        esac ;;
    *) fail "post-midline case (exit $code, want nonzero naming midline.ddl) -- $out" ;;
esac

# --- Outside the allowed roots (database/ddl, database/tests/fixtures). ---
workdir=$(mktemp -d)
printf 'select 1;\n' > "$workdir/outside.sql"
expect_roots_refuse "outside-roots DDL refused by name" "outside.sql" ./run.sh --apply dojo "$workdir/outside.sql" "$FX/ok.probe"

# --- L2: the roots guard resolves paths (.. and symlinks), not string prefixes. ---
# fixtures/run-apply/../../run.sh is database/tests/run.sh: it exists, it is
# outside the roots, and its string starts with fixtures/.
expect_roots_refuse "traversal DDL refused by name" "../.." ./run.sh --apply dojo "$FX/../../run.sh" "$FX/ok.probe"

# --- From another cwd with repo-relative paths (paths resolve before cd). ---
out=$(cd ../.. && database/tests/run.sh --apply dojo database/tests/fixtures/run-apply/a.ddl database/tests/fixtures/run-apply/b.ddl database/tests/fixtures/run-apply/ok.probe 2>&1); code=$?
if [ "$code" -ne 0 ]; then
    fail "repo-root call (exit $code, want 0) -- $out"
else
    pass "repo-root call with repo-relative paths"
fi

# --- F1: the plane takes effect. sensei.probe fails wherever dojo exists. ---
out=$(./run.sh --apply dojo "$FX/a_sensei.ddl" "$FX/sensei.probe" 2>&1); code=$?
case "$code:$out" in
    0:*) fail "sensei probe on dojo exited 0" ;;
    *sensei-plane-probe*) pass "sensei probe on dojo fails naming the plane mismatch" ;;
    *) fail "sensei probe on dojo (exit $code, want the probe error) -- $out" ;;
esac

# The positive sensei case needs the daemon database, which the CI dōjō job
# has not got: skip it by name, loudly, and run everything else.
if ! psql "$SENSEI_URL" -X -q -c 'select 1' >/dev/null 2>&1; then
    echo "self-test-apply: SKIP sensei-plane positive case — cannot reach $SENSEI_URL" >&2
else
    fp_s_before=$(fingerprint "$SENSEI_URL")
    out=$(./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe" 2>&1); code=$?
    fp_s_after=$(fingerprint "$SENSEI_URL")
    if [ "$code" -ne 0 ]; then
        fail "sensei-plane positive (exit $code, want 0) -- $out"
    elif [ "$fp_s_before" != "$fp_s_after" ]; then
        fail "sensei catalogue changed across the run ($fp_s_before / $fp_s_after)"
    elif [ "$(canaries_present "$SENSEI_URL")" != "0" ]; then
        fail "sensei canary persisted after the run"
    else
        pass "sensei-plane positive green, catalogue identical, no canaries left"
    fi
fi

# --- Nothing from any dojo case above may persist. ---
fp_final=$(fingerprint "$DOJO_URL")
if [ "$fp_final" != "$fp_before" ]; then
    fail "catalogue changed across the dojo run ($fp_before / $fp_final)"
elif [ "$(canaries_present "$DOJO_URL")" != "0" ]; then
    fail "canary functions persisted after the dojo run"
else
    pass "catalogue identical after all dojo cases"
fi

if [ "$failed" -ne 0 ]; then
    echo "self-test-apply: $failed failing case(s)" >&2
    exit 1
fi
echo "self-test-apply: all green"
