#!/usr/bin/env bash
#
# Run every SQL assertion under database/tests/ against a real Postgres.
#
# The exit status is the result — each file is run with ON_ERROR_STOP=1, so a
# raised assertion makes psql exit non-zero and this script reports it. Nothing
# greps psql's output for a word like FAILED, which would also "pass" when
# nothing ran at all.
set -uo pipefail

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
