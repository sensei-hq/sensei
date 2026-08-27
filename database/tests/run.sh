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

DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@127.0.0.1:54322/postgres}"

if ! command -v psql >/dev/null 2>&1; then
    echo "psql not found on PATH." >&2
    echo "  Homebrew keeps it keg-only: export PATH=\"\$(brew --prefix libpq)/bin:\$PATH\"" >&2
    exit 127
fi

# Prove the target is reachable before reporting any file as passing — an
# unreachable database would otherwise fail every file identically and read as
# "the schema is broken" rather than "nothing was tested".
if ! psql "$DATABASE_URL" -X -q -c 'select 1' >/dev/null 2>&1; then
    echo "cannot reach $DATABASE_URL" >&2
    echo "  is the local Supabase up?  supabase start" >&2
    exit 69
fi

# `find`, not a `**` glob: macOS ships bash 3.2, which has no `globstar` and
# quietly treats `**` as `*` — a test one directory deeper would have been
# skipped in silence while the runner still reported success.
failed=0
ran=0
while IFS= read -r file; do
    ran=$((ran + 1))
    if psql "$DATABASE_URL" -X -q -v ON_ERROR_STOP=1 -f "$file"; then
        echo "  ok    $file"
    else
        echo "  FAIL  $file" >&2
        failed=$((failed + 1))
    fi
done < <(find . -name '*.sql' -type f | sort)

if [ "$ran" -eq 0 ]; then
    echo "no test files found under database/tests/ — nothing was verified" >&2
    exit 1
fi

if [ "$failed" -ne 0 ]; then
    echo "$failed of $ran file(s) failed" >&2
    exit 1
fi

echo "$ran file(s) passed"
