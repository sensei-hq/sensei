#!/usr/bin/env bash
# The fingerprint of the schema the test template must match (#142).
#
#   scripts/test-template-fingerprint.sh              print it
#   scripts/test-template-fingerprint.sh --stamp URL  record it on URL's database
#
# One hash over the TRACKED files of database/ddl, database/import and
# database/design.yaml, as they are in the WORKING TREE — an uncommitted DDL
# edit changes it, and a gitignored local export (import/staging/
# assistant_events.jsonl) does not. `make test-template` and CI stamp the
# template with it after building; the Rust suite compares before its first
# clone (`test_db`), so a template behind the tree fails the run by name.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fingerprint=$(git ls-files -z database/ddl database/import database/design.yaml \
  | xargs -0 git hash-object \
  | git hash-object --stdin)

if [[ "${1:-}" == "--stamp" ]]; then
  url="${2:?--stamp needs a database URL}"
  db=$(psql "$url" -Atc "SELECT current_database()")
  psql "$url" -qc "COMMENT ON DATABASE \"$db\" IS 'ddl-fingerprint:$fingerprint'"
  echo "stamped $db: $fingerprint"
else
  echo "$fingerprint"
fi
