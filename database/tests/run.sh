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
cd "$(dirname "$0")"

# Each directory is a dbd scope, and runs against the database that scope
# deploys to: `dojo/` the dōjō (local Supabase), everything else the daemon's
# own `sensei` database. One target for all of them ran `gateway/` against the
# dōjō, which has no `sensei.model_capability` — a failure no fix could clear.
# DATABASE_URL keeps naming the dōjō target, as it always has.
DOJO_DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"
SENSEI_DATABASE_URL="${SENSEI_DATABASE_URL:-postgresql://localhost:5432/sensei}"

database_for() { # FILE
    case "$1" in
        ./dojo/*) printf '%s' "$DOJO_DATABASE_URL" ;;
        *) printf '%s' "$SENSEI_DATABASE_URL" ;;
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
    case "$plane" in
        dojo) target_url="$DOJO_DATABASE_URL" ;;
        sensei) target_url="$SENSEI_DATABASE_URL" ;;
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
    # autocommit, so whole-line COMMIT/ROLLBACK (any case, COMMIT WORK and
    # COMMIT TRANSACTION too) is refused. Whole-line BEGIN/END stays allowed:
    # real plpgsql bodies hold them at column 0.
    ddl_guard_re='^[[:space:]]*(commit|rollback)([[:space:]]+(work|transaction))?[[:space:]]*;[[:space:]]*$'
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
    if ! psql "$target_url" -X -q -c 'select 1' >/dev/null 2>&1; then
        echo "cannot reach $target_url" >&2
        exit 69
    fi
    psql_args=(-X -q -v ON_ERROR_STOP=1 -c "begin" -c "select set_config('run_apply.txn','on',true)")
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
        psql_args+=(-f "$fa" -c "do \$\$ begin if current_setting('run_apply.txn', true) is distinct from 'on' then raise exception 'run.sh --apply: $f ended the transaction'; end if; end \$\$;")
    done
    psql_args+=(-f "$ta" -c "rollback")
    psql "$target_url" "${psql_args[@]}"
    exit $?
fi

# `run.sh <scope>` runs one scope's directory and asks only for that scope's
# database — a CI job that stands up the dōjō has no daemon database, and the
# dōjō's access rules are most of this suite. An unknown scope is refused, so a
# typo cannot run nothing and pass. No argument: every scope, every database.
search=.
urls=("$DOJO_DATABASE_URL" "$SENSEI_DATABASE_URL")
if [ $# -gt 0 ]; then
    if [ ! -d "./$1" ]; then
        echo "no such test scope: $1 (one of: $(find . -mindepth 1 -maxdepth 1 -type d | sed 's#^\./##' | sort | tr '\n' ' '))" >&2
        exit 2
    fi
    search="./$1"
    urls=("$(database_for "./$1/x")")
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
        echo "  the dōjō tests need the local Supabase (supabase start); the rest, the daemon's database" >&2
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
