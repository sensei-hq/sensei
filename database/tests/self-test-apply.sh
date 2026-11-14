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

cd "$(dirname "$0")" || exit 1

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
expect_refuse "DDL with abort; refused" "txn-abort.ddl" ./run.sh --apply dojo "$FX/txn-abort.ddl" "$FX/ok.probe"
# Test files get a commit-only guard (F5): begin;/rollback; stay allowed, and
# noop.ddl persists nothing even if the guard is missing and it commits.
expect_refuse "test file with commit; refused" "commits.probe" ./run.sh --apply dojo "$FX/noop.ddl" "$FX/commits.probe"
expect_refuse "test file with COMMIT WORK; refused" "commits-work.probe" ./run.sh --apply dojo "$FX/noop.ddl" "$FX/commits-work.probe"

# --- sensei-target lib: pure unit tests (no database needed). ---
# Each case fails "not implemented" until lib/sensei-target.sh provides the
# function, so the suite is red on the unmodified tree.
if [ -f ./lib/sensei-target.sh ]; then
    # shellcheck disable=SC1091
    . ./lib/sensei-target.sh
fi
unit() { # <desc> <fn> <want-exit> <arg...>: fn exists and exits want.
    desc="$1"; fn="$2"; want="$3"; shift 3
    if ! command -v "$fn" >/dev/null 2>&1; then
        fail "$desc ($fn not implemented)"
        return
    fi
    if "$fn" "$@" >/dev/null 2>&1; then code=0; else code=$?; fi
    if [ "$code" -ne "$want" ]; then
        fail "$desc (exit $code, want $want)"
    else
        pass "$desc"
    fi
}
unit_out() { # <desc> <fn> <want-out> <arg...>: fn exists and prints want-out.
    desc="$1"; fn="$2"; want="$3"; shift 3
    if ! command -v "$fn" >/dev/null 2>&1; then
        fail "$desc ($fn not implemented)"
        return
    fi
    got="$("$fn" "$@" 2>/dev/null)" || { fail "$desc ($fn failed)"; return; }
    if [ "$got" != "$want" ]; then
        fail "$desc (got '$got', want '$want')"
    else
        pass "$desc"
    fi
}
LIVE_MSG='refusing the live daemon DB'
NOLANE_MSG='cannot derive a lane database'
unit "live: host:port/dbname" sensei_is_live 0 'postgresql://localhost:5432/sensei'
unit "live: default port" sensei_is_live 0 'postgresql://localhost/sensei'
unit "live: 127.0.0.1" sensei_is_live 0 'postgresql://127.0.0.1:5432/sensei'
unit "live: 127.x loopback" sensei_is_live 0 'postgresql://127.0.0.2:5432/sensei'
unit "live: userinfo + query" sensei_is_live 0 'postgresql://u:p@localhost:5432/sensei?sslmode=require'
unit "live: userinfo with @" sensei_is_live 0 'postgresql://u@example.com:p@localhost:5432/sensei'
unit "live: case-insensitive" sensei_is_live 0 'POSTGRESQL://LOCALHOST:5432/sensei'
unit "live: unix socket, empty host" sensei_is_live 0 'postgresql:///sensei'
unit "live: encoded socket host" sensei_is_live 0 'postgresql://%2Fvar%2Frun%2Fpostgresql/sensei'
unit "live: daemon port" sensei_is_live 0 'postgresql://localhost:7744/sensei'
unit "live: trailing slash" sensei_is_live 0 'postgresql://localhost:5432/sensei/'
unit "live: bracket ipv6 loopback" sensei_is_live 0 'postgresql://[::1]:5432/sensei'
unit "live: 0.0.0.0 routes to localhost" sensei_is_live 0 'postgresql://0.0.0.0:5432/sensei'
unit "live: trailing-dot localhost" sensei_is_live 0 'postgresql://localhost.:5432/sensei'
unit "live: dbname case folds" sensei_is_live 0 'postgresql://localhost:5432/SENSEI'
# N2: libpq's dbname chain PGDATABASE → PGUSER → USER (characterization +
# new pins; subshells unset first so the runner's env can't leak in).
if ( unset PGDATABASE PGUSER; USER=sensei sensei_is_live 'postgresql://localhost:5432/' ); then pass "live: USER names sensei"; else fail "live: USER names sensei"; fi
if ( unset PGDATABASE PGUSER; USER=nobody PGUSER=sensei sensei_is_live 'postgresql://localhost:5432/' ); then pass "live: PGUSER names sensei"; else fail "live: PGUSER names sensei"; fi
if ( unset PGDATABASE PGUSER; USER=nobody PGDATABASE=sensei sensei_is_live 'postgresql://localhost:5432/' ); then pass "live: PGDATABASE names sensei"; else fail "live: PGDATABASE names sensei"; fi
if ( unset PGDATABASE PGUSER; USER=sensei PGDATABASE=other sensei_is_live 'postgresql://localhost:5432/' ); then fail "not live: PGDATABASE wins over USER"; else pass "not live: PGDATABASE wins over USER"; fi
unit "not live: lane db name" sensei_is_live 1 'postgresql://localhost:5432/sensei_test_l3'
unit "not live: other port" sensei_is_live 1 'postgresql://localhost:5433/sensei'
unit "not live: remote host" sensei_is_live 1 'postgresql://db.internal:5432/sensei'
unit "not live: other db on socket" sensei_is_live 1 'postgresql:///sensei_test_x'
unit_out "lane name from lane branch" sensei_lane_name 'l3_relay' 'lane/L3-relay'
unit_out "lane name from plain branch" sensei_lane_name 'develop' 'develop'
unit_out "lane name sanitizes" sensei_lane_name 'feature_x_y' 'feature/x-y'
unit_out "lane name truncates to 51" sensei_lane_name 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
unit "lane name refuses empty" sensei_lane_name 1 ''

# --- Live-DB refusal (no database needed: refused before any touch). ---
# TEST_DATABASE_URL is scrubbed where SENSEI_DATABASE_URL is under test: the
# caller's environment may export it, and it takes precedence by design.
LIVE='postgresql://localhost:5432/sensei'
expect_refuse "apply sensei refuses SENSEI_DATABASE_URL=live" "$LIVE_MSG" env -u TEST_DATABASE_URL SENSEI_DATABASE_URL="$LIVE" ./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe"
expect_refuse "apply sensei refuses TEST_DATABASE_URL=live" "$LIVE_MSG" env TEST_DATABASE_URL="$LIVE" ./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe"
expect_refuse "apply sensei refuses 127.0.0.1 live" "$LIVE_MSG" env -u TEST_DATABASE_URL SENSEI_DATABASE_URL='postgresql://127.0.0.1:5432/sensei' ./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe"
expect_refuse "apply sensei refuses daemon-port live" "$LIVE_MSG" env -u TEST_DATABASE_URL SENSEI_DATABASE_URL='postgresql://localhost:7744/sensei' ./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe"
expect_refuse "plain sensei scope refuses live" "$LIVE_MSG" env -u TEST_DATABASE_URL SENSEI_DATABASE_URL="$LIVE" ./run.sh sensei
# Guard: an explicit non-live SENSEI_DATABASE_URL keeps routing there (unreachable
# port → 69 naming it, not a refusal and not the live default).
out=$(env -u TEST_DATABASE_URL SENSEI_DATABASE_URL='postgresql://localhost:5499/sensei_xxx' ./run.sh sensei 2>&1); code=$?
case "$code:$out" in
    69:*5499*) pass "explicit non-live SENSEI_DATABASE_URL still routes there" ;;
    *) fail "explicit SENSEI_DATABASE_URL routing (exit $code) -- $out" ;;
esac

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
# No live SENSEI_URL default: the sensei plane runs on scratch lane DBs only
# (see the sensei-plane section below).
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
expect_refuse "DDL with abort; refused (no-trace re-run)" "txn-abort.ddl" ./run.sh --apply dojo "$FX/txn-abort.ddl" "$FX/ok.probe"
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

# --- F1: a mid-line commit in a DDL file trips the trap, not just the marker. ---
# The trap is installed before the DDL files (L-a), so the commit aborts with
# the trap's refusal. A trap installed after the DDL would let it commit and
# only the marker would fire.
out=$(./run.sh --apply dojo "$FX/midcommit.ddl" "$FX/ok.probe" 2>&1); code=$?
if [ "$code" -eq 0 ]; then
    fail "midline commit in DDL exited 0"
else
    case "$out" in
        *"wrapper transaction was committed"*) pass "midline commit in DDL trips the trap" ;;
        *) fail "midline commit in DDL (exit $code, want the commit-trap refusal) -- $out" ;;
    esac
fi

# --- F2: the tail of a DDL file after a rollback stays transactional. ---
# With AUTOCOMMIT off, psql opens an implicit transaction for the tail, so a
# savepoint neither errors nor persists; the marker then names the file. The
# savepoint needs a transaction block and creates nothing.
out=$(./run.sh --apply dojo "$FX/rb-tail.ddl" "$FX/ok.probe" 2>&1); code=$?
case "$code:$out" in
    0:*) fail "rollback-tail DDL exited 0" ;;
    *"can only be used in transaction blocks"*) fail "rollback tail ran outside a transaction -- $out" ;;
    *rb-tail.ddl*) pass "rollback tail stays transactional, file named" ;;
    *) fail "rollback tail (exit $code, want nonzero naming rb-tail.ddl) -- $out" ;;
esac

# --- F1: a quote in a DDL path is escaped in the marker. ---
# A symlink keeps quoted names out of the repo: the roots guard reads the
# real path (admitted — it resolves into fixtures/), while the marker
# splices the path as given, quote and all. midline.ddl persists nothing
# even unescaped.
qdir=$(mktemp -d)
ln -s "$PWD/$FX/midline.ddl" "$qdir/it's.ddl"
out=$(./run.sh --apply dojo "$qdir/it's.ddl" "$FX/ok.probe" 2>&1); code=$?
rm -rf "$qdir"
case "$code:$out" in
    0:*) fail "quoted DDL path exited 0" ;;
    *"syntax error"*) fail "quoted DDL path breaks the marker -- $out" ;;
    *"it's.ddl ended the transaction"*) pass "quoted DDL path named by the marker" ;;
    *) fail "quoted DDL path (exit $code, want the marker naming it's.ddl) -- $out" ;;
esac

# --- F2: a rolled-back tail that then commits persists nothing. ---
# The wrapper runs read-write under a read-only session default, so any
# transaction after the wrapper ends is read-only: the tail's write errors
# out under ON_ERROR_STOP instead of committing.
out=$(./run.sh --apply dojo "$FX/rb-commit-tail.ddl" "$FX/ok.probe" 2>&1); code=$?
case "$code:$out" in
    0:*) fail "rollback-commit tail exited 0" ;;
    *"read-only transaction"*) pass "rollback-commit tail refused read-only" ;;
    *) fail "rollback-commit tail (exit $code, want a read-only refusal) -- $out" ;;
esac

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

# --- Sensei plane runs on a scratch lane DB, never live. ---
# Scratch: createdb + the sensei schema (a_sensei.ddl needs it) + the routing
# marker (proves WHERE the run went). Dropped afterwards; the name carries the
# PID and the drop is pattern-guarded. No local postgres (the CI dōjō job):
# skip loudly, run everything else.
SCRATCH="sensei_test_selfapply_$$"
case "$SCRATCH" in
    sensei_test_selfapply_*) : ;;
    *) echo "self-test-apply: refusing to use scratch name $SCRATCH" >&2; exit 1 ;;
esac
scratch_up=0
if psql -h localhost -X -q -c 'select 1' postgres >/dev/null 2>&1 \
   && createdb -h localhost "$SCRATCH" >/dev/null 2>&1 \
   && psql -h localhost -X -q -c 'create schema sensei; create table public.__apply_route_marker (x int);' "$SCRATCH" >/dev/null 2>&1; then
    scratch_up=1
else
    dropdb --if-exists -h localhost "$SCRATCH" >/dev/null 2>&1 || true
fi
if [ "$scratch_up" -ne 1 ]; then
    echo "self-test-apply: SKIP sensei-plane cases — no local postgres for a scratch DB" >&2
else
    SCRATCH_URL="postgresql://localhost:5432/$SCRATCH"
    fp_s_before=$(fingerprint "$SCRATCH_URL")
    # Routing proof: the marker lives ONLY in scratch, so exit 0 proves
    # --apply sensei honoured TEST_DATABASE_URL (red while it ignores it).
    out=$(env -u SENSEI_DATABASE_URL TEST_DATABASE_URL="$SCRATCH_URL" ./run.sh --apply sensei "$FX/noop.ddl" "$FX/route-sensei.probe" 2>&1); code=$?
    if [ "$code" -ne 0 ]; then
        fail "sensei routing proof (exit $code, want 0) -- $out"
    else
        pass "sensei routing proof honours TEST_DATABASE_URL"
    fi
    # End-to-end on the lane plane: real DDL + plane probe, green.
    out=$(env -u SENSEI_DATABASE_URL TEST_DATABASE_URL="$SCRATCH_URL" ./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe" 2>&1); code=$?
    if [ "$code" -ne 0 ]; then
        fail "sensei-plane positive (exit $code, want 0) -- $out"
    else
        pass "sensei-plane positive green"
    fi
    fp_s_after=$(fingerprint "$SCRATCH_URL")
    if [ "$fp_s_before" != "$fp_s_after" ]; then
        fail "scratch catalogue changed across the runs ($fp_s_before / $fp_s_after)"
    elif [ "$(canaries_present "$SCRATCH_URL")" != "0" ]; then
        fail "sensei canary persisted after the runs"
    else
        pass "scratch catalogue identical, no canaries left"
    fi
    dropdb -h localhost "$SCRATCH" >/dev/null 2>&1 || fail "could not drop $SCRATCH"
fi

# --- Auto-create: without either var, run.sh derives + creates the lane DB. ---
# Hermetic via GIT_DIR: a fake repo on lane/Selfapply<PID> (unique name, safe
# to drop) or detached HEAD (must refuse). dbd missing: skip loudly.
if ! command -v dbd >/dev/null 2>&1; then
    echo "self-test-apply: SKIP auto-create cases — dbd not found on PATH" >&2
elif ! psql -h localhost -X -q -c 'select 1' postgres >/dev/null 2>&1; then
    echo "self-test-apply: SKIP auto-create cases — no local postgres" >&2
else
    fakerepo=$(mktemp -d)/repo
    mkdir -p "$fakerepo" && env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git init -qb "lane/Selfapply$$" "$fakerepo" >/dev/null 2>&1
    AUTODB="sensei_test_selfapply$$"
    case "$AUTODB" in
        sensei_test_selfapply*) : ;;
        *) echo "self-test-apply: refusing to use auto name $AUTODB" >&2; exit 1 ;;
    esac
    dropdb --if-exists -h localhost "$AUTODB" >/dev/null 2>&1 || true
    out=$(env -u TEST_DATABASE_URL -u SENSEI_DATABASE_URL -u GIT_WORK_TREE -u GIT_INDEX_FILE GIT_DIR="$fakerepo/.git" ./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe" 2>&1); code=$?
    if [ "$code" -ne 0 ]; then
        fail "auto-create run (exit $code, want 0) -- $out"
    elif ! psql -h localhost -lqt 2>/dev/null | cut -d'|' -f1 | grep -qw "$AUTODB"; then
        fail "auto-create run left no $AUTODB behind"
    else
        pass "auto-create derives, deploys and runs on $AUTODB"
    fi
    dropdb --if-exists -h localhost "$AUTODB" >/dev/null 2>&1 || true
    rm -rf "$(dirname "$fakerepo")"
    # N1: a lost createdb race is rescued — the loser re-probes and proceeds
    # instead of exiting 69. Deterministic: a fake createdb that creates the
    # DB (as the race winner did) yet exits 1.
    racedb="sensei_test_selfapply_race$$"
    case "$racedb" in
        sensei_test_selfapply_*) : ;;
        *) echo "self-test-apply: refusing to use race name $racedb" >&2; exit 1 ;;
    esac
    dropdb --if-exists -h localhost "$racedb" >/dev/null 2>&1 || true
    fakebin=$(mktemp -d)
    printf '#!/usr/bin/env bash\npsql -h localhost postgres -qc "CREATE DATABASE \\"$3\\"" >/dev/null 2>&1\nexit 1\n' > "$fakebin/createdb"
    chmod +x "$fakebin/createdb"
    if PATH="$fakebin:$PATH" sensei_ensure_db "postgresql://localhost:5432/$racedb" >/dev/null 2>&1 \
        && psql -h localhost -X -q -c 'select count(*) from relay.crews' "postgresql://localhost:5432/$racedb" >/dev/null 2>&1; then
        pass "lost createdb race rescued and deployed on $racedb"
    else
        fail "lost createdb race not rescued on $racedb"
    fi
    dropdb --if-exists -h localhost "$racedb" >/dev/null 2>&1 || true
    rm -rf "$fakebin"
    # Detached HEAD derives nothing: refuse, naming TEST_DATABASE_URL.
    fakerepo2=$(mktemp -d)/repo
    mkdir -p "$fakerepo2" && env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git init -qb main "$fakerepo2" >/dev/null 2>&1 \
        && env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git -C "$fakerepo2" -c user.email=t@t -c user.name=t commit -q --allow-empty -m x >/dev/null 2>&1 \
        && env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git -C "$fakerepo2" checkout -q --detach >/dev/null 2>&1
    out=$(env -u TEST_DATABASE_URL -u SENSEI_DATABASE_URL -u GIT_WORK_TREE -u GIT_INDEX_FILE GIT_DIR="$fakerepo2/.git" ./run.sh --apply sensei "$FX/a_sensei.ddl" "$FX/sensei.probe" 2>&1); code=$?
    case "$code:$out" in
        2:*"$NOLANE_MSG"*) pass "detached HEAD refuses naming TEST_DATABASE_URL" ;;
        *) fail "detached HEAD (exit $code, want 2 + $NOLANE_MSG) -- $out" ;;
    esac
    rm -rf "$(dirname "$fakerepo2")"
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
