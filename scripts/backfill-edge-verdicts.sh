#!/usr/bin/env bash
#
# Backfill sensei.edges.resolved_via / unresolved_reason from the per-use
# verdicts already sitting in props.occurrences.
#
# WHY A BACKFILL EXISTS AT ALL. Both columns are derived at write time now — see
# `insert_edge_with_props` — but every edge written before that fix has them
# NULL, and NULL is indistinguishable from "the writer recorded no verdict".
# Only a pass over the existing rows can tell those two apart.
#
# RUN IT WHEN THE INDEXER IS IDLE. The first attempt at this ran against a live
# indexing daemon and pushed its `INSERT INTO sensei.edges` latency from ~30ms to
# 42 SECONDS. The table is ~4.2 GB; a bulk UPDATE and a live walk do not share it
# politely.
#
#   sensei daemon stop     # or wait for indexing to finish
#   scripts/backfill-edge-verdicts.sh
#
# KEYSET PAGINATION ON THE PRIMARY KEY, not a predicate on a computed value. The
# first version batched on `left(id::text,1)`, which no index can serve, so each
# of its sixteen batches was a full scan of the heap — sixteen passes to do one
# pass of work. This walks the pk in id order, so every batch is an index range.
#
# `modified_at` is deliberately NOT touched: this derives a column from data the
# row already carried, and stamping four million edges as modified today would
# destroy the one signal that says when an edge actually changed.
set -uo pipefail

DB="${DATABASE_URL:-postgres://$USER@localhost:5432/sensei}"
BATCH="${BATCH:-20000}"
PAUSE="${PAUSE:-0.2}"
PSQL=(psql "$DB" -v ON_ERROR_STOP=1 -tAq)

busy=$("${PSQL[@]}" -c "select count(*) from pg_stat_activity
                         where datname = current_database()
                           and query like 'INSERT INTO sensei.edges%'")
if [ "${busy:-0}" -gt 0 ]; then
  echo "REFUSING: $busy indexer insert(s) are running against sensei.edges." >&2
  echo "A bulk update alongside a live walk starves it. Stop the daemon first." >&2
  exit 1
fi

total=$("${PSQL[@]}" -c "select count(*) from sensei.edges")
echo "backfilling $total edges in batches of $BATCH"

cursor="00000000-0000-0000-0000-000000000000"
done_rows=0
while :; do
  read -r n last < <("${PSQL[@]}" -F' ' -c "
    WITH b AS (
      SELECT id FROM sensei.edges WHERE id > '$cursor' ORDER BY id LIMIT $BATCH
    ), u AS (
      UPDATE sensei.edges e
         SET resolved_via      = sensei.edge_verdict(e.props->'occurrences','code_graph_rung','rung'),
             unresolved_reason = sensei.edge_verdict(e.props->'occurrences','code_graph','reason')
        FROM b WHERE e.id = b.id
      RETURNING e.id
    )
    SELECT count(*), coalesce(max(id)::text, '')  FROM u")
  [ "${n:-0}" -eq 0 ] && break
  cursor="$last"
  done_rows=$((done_rows + n))
  printf '  %s / %s\r' "$done_rows" "$total"
  sleep "$PAUSE"
done

echo
"${PSQL[@]}" -F'|' -c "
  SELECT count(*) FILTER (WHERE resolved_via IS NOT NULL)      AS placed_with_rung
       , count(*) FILTER (WHERE unresolved_reason IS NOT NULL) AS missed_with_reason
       , count(*) FILTER (WHERE resolved_via IS NULL AND unresolved_reason IS NULL) AS no_verdict
       , count(*) AS total
    FROM sensei.edges"
echo "backfill complete"
