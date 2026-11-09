#!/usr/bin/env bash
#
# Run every SQL assertion under database/tests/ against a real Postgres.
#
# The exit status is the result — each file is run with ON_ERROR_STOP=1, so a
# raised assertion makes psql exit non-zero and this script reports it. Nothing
# greps psql's output for a word like FAILED, which would also "pass" when
# nothing ran at all.
set -uo pipefail

# `--apply` resolves its paths against the caller's cwd, so this is captured
# before the cd below.
caller_pwd="$PWD"
cd "$(dirname "$0")" || exit 1

# shellcheck disable=SC1091
. ./lib/sensei-target.sh

# Each directory is a dbd scope, and runs against the database that scope
# deploys to: `dojo/` the dōjō (local Supabase), everything else a lane-owned
# test database — never the live daemon DB (L0 ruling 2026-11-05: the stray
# `relay` schema). TEST_DATABASE_URL wins, then an explicit non-live
# SENSEI_DATABASE_URL, then a branch-derived sensei_test_<lane> that run.sh
# creates from the DDL tree; dbname `sensei` on :5432/:7744 is refused with a
# clear message whatever named it. One target for all of them ran `gateway/`
# against the dōjō, which has no `sensei.model_capability` — a failure no fix
# could clear. DATABASE_URL keeps naming the dōjō target, as it always has.
DOJO_DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"
SENSEI_DATABASE_URL="${SENSEI_DATABASE_URL:-}"
SENSEI_URL_RESOLVED=""

database_for() { # FILE
    case "$1" in
        ./dojo/*) printf '%s' "$DOJO_DATABASE_URL" ;;
        *) printf '%s' "$SENSEI_URL_RESOLVED" ;;
    esac
}

# `run.sh --apply <dojo|sensei> <ddl>... <test>`: apply unmerged DDL files and
# run a test in ONE psql session inside a rolled-back wrapper transaction, so
# lanes test DDL without converging a shared database (R1). Exit 2 + usage on
# any validation failure, before touching a database; otherwise the test's own
# result. DDL comes from database/ddl/ or database/tests/fixtures/ only.
if [ "${1:-}" = "--apply" ]; then
    shift
    apply_usage() { echo "usage: run.sh --apply <dojo|sensei> <ddl>... <test> -- $1" >&2; exit 2; }
    [ $# -ge 3 ] || apply_usage "want a plane, at least one DDL file and a test file"
    plane="$1"; shift
    sensei_plane=0
    case "$plane" in
        dojo) target_url="$DOJO_DATABASE_URL" ;;
        sensei) sensei_plane=1 ;;
        *) apply_usage "unknown plane: $plane" ;;
    esac
    # Last argument is the test file; the rest are DDL files.
    nargs=$#
    ddl_list=()
    test_arg=""
    idx=1
    for a in "$@"; do
        if [ "$idx" -eq "$nargs" ]; then test_arg="$a"; else ddl_list+=("$a"); fi
        idx=$((idx + 1))
    done
    abspath() {
        case "$1" in
            /*) printf '%s' "$1" ;;
            *) printf '%s' "$caller_pwd/$1" ;;
        esac
    }
    ddl_root="$(realpath ../ddl)"
    fx_root="$(realpath fixtures)"
    # A DDL file that ends the wrapper early makes every file after it
    # autocommit, so whole-line COMMIT/ROLLBACK/ABORT (any case, WORK and
    # TRANSACTION forms too) is refused. Whole-line BEGIN/END stays allowed:
    # real plpgsql bodies hold them at column 0.
    ddl_guard_re='^[[:space:]]*(commit|rollback|abort)([[:space:]]+(work|transaction))?[[:space:]]*;[[:space:]]*$'
    # Test files keep their own BEGIN/ROLLBACK (every suite file has them), but
    # a COMMIT in a test would persist the applied DDL: commit forms refused.
    test_guard_re='^[[:space:]]*commit([[:space:]]+(work|transaction))?[[:space:]]*;[[:space:]]*$'
    for f in "${ddl_list[@]}"; do
        fa="$(abspath "$f")"
        [ -f "$fa" ] || apply_usage "$f: no such file"
        rp="$(realpath "$fa")"
        case "$rp" in
            "$ddl_root"/*|"$fx_root"/*) : ;;
            *) apply_usage "$f is outside the allowed roots (database/ddl, database/tests/fixtures)" ;;
        esac
        if grep -q -i -E "$ddl_guard_re" "$fa"; then
            apply_usage "$f manages its own transaction"
        fi
    done
    ta="$(abspath "$test_arg")"
    [ -f "$ta" ] || apply_usage "$test_arg: no such file"
    if grep -q -i -E "$test_guard_re" "$ta"; then
        apply_usage "$test_arg manages its own transaction (commit in test file)"
    fi
    if ! command -v psql >/dev/null 2>&1; then
        echo "psql not found on PATH." >&2
        echo "  Homebrew keeps it keg-only: export PATH=\"\$(brew --prefix libpq)/bin:\$PATH\"" >&2
        exit 127
    fi
    # The sensei target resolves after file validation (no database is created
    # for a call that usage already refuses) and before the reachability probe.
    if [ "$sensei_plane" -eq 1 ]; then
        target_url="$(sensei_resolve_url)" || exit $?
    fi
    if ! psql "$target_url" -X -q -c 'select 1' >/dev/null 2>&1; then
        echo "cannot reach $target_url" >&2
        exit 69
    fi
    # AUTOCOMMIT=off: after a rollback inside a DDL file, psql opens an
    # implicit transaction for the file's tail instead of autocommitting it,
    # so the tail persists nothing and the per-file marker still fires (F2).
    # The wrapper opens read-write under a read-only session default
    # (PGOPTIONS below): any transaction after the wrapper ends is read-only,
    # so a tail that then commits mid-line errors out instead of persisting.
    psql_args=(-X -q -v ON_ERROR_STOP=1 -v AUTOCOMMIT=off -c "begin read write" -c "select set_config('run_apply.txn','on',true)")
    # The commit trap (L-a): a deferred constraint trigger in pg_temp turns ANY
    # commit of the wrapper — commit, end, mid-line, in a DDL file or a test —
    # into an abort. Rolls back with the wrapper; temp-only, nothing persists.
    psql_args+=(-c "create temp table __apply_commit_guard (x int)")
    psql_args+=(-c "create function pg_temp.__apply_refuse_commit() returns trigger language plpgsql as \$g\$ begin raise exception 'run.sh --apply: the wrapper transaction was committed; refused, nothing persists'; end \$g\$")
    psql_args+=(-c "create constraint trigger __apply_commit_guard after insert on pg_temp.__apply_commit_guard deferrable initially deferred for each row execute function pg_temp.__apply_refuse_commit()")
    psql_args+=(-c "insert into pg_temp.__apply_commit_guard values (1)")
    for f in "${ddl_list[@]}"; do
        fa="$(abspath "$f")"
        # The per-file marker (L3/L-b): a ROLLBACK in a DDL file ends the
        # wrapper without firing the trap, so every file re-checks that the
        # transaction-local setting survived, naming the file that ended it.
        # The name is spliced into a SQL literal, so a quote in it is doubled
        # (through $q: a backslash-quote replacement stays literal in ${//}).
        q="'"
        f_esc="${f//$q/$q$q}"
        psql_args+=(-f "$fa" -c "do \$\$ begin if current_setting('run_apply.txn', true) is distinct from 'on' then raise exception 'run.sh --apply: $f_esc ended the transaction'; end if; end \$\$;")
    done
    psql_args+=(-f "$ta" -c "rollback")

    # Read-only by default for every transaction but the wrapper's own. Via
    # PGOPTIONS, not a SET: a SET inside the wrapper is undone by the same
    # rollback this guards against (r3 F2).
    PGOPTIONS="${PGOPTIONS:+$PGOPTIONS }-c default_transaction_read_only=on" psql "$target_url" "${psql_args[@]}"
    exit $?
fi

# `run.sh <scope>` runs one scope's directory and asks only for that scope's
# database — a CI job that stands up the dōjō has no daemon database, and the
# dōjō's access rules are most of this suite. An unknown scope is refused, so a
# typo cannot run nothing and pass. No argument: every scope, every database.
search=.
if [ $# -gt 0 ]; then
    if [ ! -d "./$1" ]; then
        echo "no such test scope: $1 (one of: $(find . -mindepth 1 -maxdepth 1 -type d | sed 's#^\./##' | sort | tr '\n' ' '))" >&2
        exit 2
    fi
    search="./$1"
    # A dojo-only run never resolves (and never creates) the sensei target.
    case "$search" in
        ./dojo) urls=("$DOJO_DATABASE_URL") ;;
        *) SENSEI_URL_RESOLVED="$(sensei_resolve_url)" || exit $?; urls=("$SENSEI_URL_RESOLVED") ;;
    esac
else
    SENSEI_URL_RESOLVED="$(sensei_resolve_url)" || exit $?
    urls=("$DOJO_DATABASE_URL" "$SENSEI_URL_RESOLVED")
fi

if ! command -v psql >/dev/null 2>&1; then
    echo "psql not found on PATH." >&2
    echo "  Homebrew keeps it keg-only: export PATH=\"\$(brew --prefix libpq)/bin:\$PATH\"" >&2
    exit 127
fi

# Prove the target is reachable before reporting any file as passing — an
# unreachable database would otherwise fail every file identically and read as
# "the schema is broken" rather than "nothing was tested".
for url in "${urls[@]}"; do
    if ! psql "$url" -X -q -c 'select 1' >/dev/null 2>&1; then
        echo "cannot reach $url" >&2
        echo "  the dōjō tests need the local Supabase (supabase start); the rest, a lane-owned test database (TEST_DATABASE_URL, or a branch-derived sensei_test_<lane>)" >&2
        exit 69
    fi
done

# `find`, not a `**` glob: macOS ships bash 3.2, which has no `globstar` and
# quietly treats `**` as `*` — a test one directory deeper would have been
# skipped in silence while the runner still reported success.
failed=0
ran=0
while IFS= read -r file; do
    ran=$((ran + 1))
    if psql "$(database_for "$file")" -X -q -v ON_ERROR_STOP=1 -f "$file"; then
        echo "  ok    $file"
    else
        echo "  FAIL  $file" >&2
        failed=$((failed + 1))
    fi
done < <(find "$search" -name '*.sql' -type f | sort)

if [ "$ran" -eq 0 ]; then
    echo "no test files found under database/tests/ — nothing was verified" >&2
    exit 1
fi

if [ "$failed" -ne 0 ]; then
    echo "$failed of $ran file(s) failed" >&2
    exit 1
fi

echo "$ran file(s) passed"
