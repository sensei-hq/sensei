#!/usr/bin/env bash
# Essential backup of the sensei DB (#181): ONLY the tables whose contents cannot
# be rebuilt from the source tree. The Makefile's `db-backup-essential` target
# documents which tables and why; this script is how it runs.
#
#   SKIP_DB_BACKUP=1   skip it (iteration; says so)
#   BACKUP_DIR         where snapshots go   (default database/backup/essential)
#   BACKUP_KEEP        snapshots kept       (default 2)
#   DATABASE_URL       source database      (default postgres://localhost/sensei)
#
# Writes <BACKUP_DIR>/<timestamp>/<schema>/<table>.jsonl via `dbd export`.
#
# Two rules, each measured:
#   * PRUNE AFTER WRITING. Pruning first kept KEEP+1 snapshots at rest — three
#     ~4 GB copies (12 GB, 2026-10-19). Afterwards there are KEEP at rest, and
#     the previous snapshot is intact while the new one is written, so a backup
#     killed halfway still leaves a good copy.
#   * A FAILED EXPORT FAILS THE BACKUP. Each export used to end `|| echo skip`
#     with its error discarded, so a snapshot missing `sessions` reported
#     "complete". Now the partial snapshot is removed, the earlier ones are left
#     alone, and the exit is non-zero, naming the tables that failed.
set -euo pipefail

if [[ "${SKIP_DB_BACKUP:-}" == "1" ]]; then
  echo "db-backup-essential: skipped (SKIP_DB_BACKUP=1)"
  exit 0
fi

dir="${BACKUP_DIR:-database/backup/essential}"
keep="${BACKUP_KEEP:-2}"
url="${DATABASE_URL:-postgres://localhost/sensei}"

if ! psql "$url" -c "SELECT 1" >/dev/null 2>&1; then
  echo "db-backup-essential: sensei DB not present — nothing to back up"
  exit 0
fi

tables=(
  activity.assistant_events activity.sessions activity.transcript_turns activity.turns
  activity.session_process_evidence
  sensei.memories sensei.memory_outcomes sensei.tool_insights
  sensei.playbook_rules sensei.consolidated_rulesets
  inference.detected_patterns inference.recommendations
  inference.reasoning_traces inference.corrections inference.drift_items
  sensei.workspaces sensei.folders sensei.folders_to_watch
  sensei.repositories sensei.folder_path_aliases
)

out="$dir/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out"
echo "db-backup-essential: → $out"

# The table a release renames (#307 R3: sensei.projects → sensei.workspaces).
# The backup runs BEFORE the upgrade, with the new design but the database as it
# still is: the design no longer knows the old name and the database does not
# have the new one yet. Save it under the name the database has, so the snapshot
# is the database as it was. Neither name present is a failure, never a skip.
exists() { [ "$(psql "$url" -Atc "select to_regclass('$1') is not null")" = "t" ]; }
export_renamed() { # NEW OLD
  if exists "$1"; then
    DATABASE_URL="$url" dbd export -n "$1" --format jsonl --output "$out" --source database 2>&1 >/dev/null
  elif exists "$2"; then
    mkdir -p "$out/${2%%.*}" &&
      psql "$url" -Atc "\\copy (select row_to_json(t) from $2 t) to '$out/${2%%.*}/${2#*.}.jsonl'" 2>&1 >/dev/null
  else
    echo "neither $1 nor $2 exists"; return 1
  fi
}

failed=()
for t in "${tables[@]}"; do
  if [ "$t" = "sensei.workspaces" ]; then
    if ! err=$(export_renamed sensei.workspaces sensei.projects); then
      echo "db-backup-essential: export of $t failed: $err" >&2
      failed+=("$t")
    fi
    continue
  fi
  if ! err=$(DATABASE_URL="$url" dbd export -n "$t" --format jsonl --output "$out" --source database 2>&1 >/dev/null); then
    echo "db-backup-essential: export of $t failed: $err" >&2
    failed+=("$t")
  fi
done

if (( ${#failed[@]} > 0 )); then
  rm -rf "$out"
  echo "db-backup-essential: FAILED — ${#failed[@]} table(s) not exported: ${failed[*]}. The partial snapshot was removed; earlier snapshots are untouched." >&2
  exit 1
fi

# Only now, with a complete new snapshot on disk, drop the oldest beyond KEEP.
ls -1t "$dir" | tail -n +"$((keep + 1))" | while read -r d; do rm -rf "${dir:?}/$d"; done

echo "db-backup-essential: complete — $(du -sh "$out" | awk '{print $1}')"
