set search_path to sensei, extensions;

create or replace view folder_completeness as
with recursive local as (
    -- Per-folder facts ONLY. `expected_files` is written by the walk that
    -- counted them (the sole point that knows the number); `decided` counts the
    -- files rows that reached a verdict, where a deliberate skip
    -- (binary_content, invalid_utf8) is a verdict just as much as an index is.
    --
    -- `parsed_at`, NOT `indexed_at`. `indexed_at` is NOT NULL DEFAULT now(),
    -- so `indexed_at is not null` is a TAUTOLOGY: every file row counted as
    -- decided the instant the walk created it, and this view reported
    -- subtree_complete on all 18 folders of a repo with 2,305 files and zero
    -- parsed — the exact vacuous count the comment below warns against.
    -- `parsed_at` is null until a parse actually records an outcome.
    select f.id
         , f.parent_id
         , f.status
         , (f.props->>'expected_files')::bigint as expected
         , count(s.*) filter (
               where s.parsed_at is not null or s.skip_reason is not null
           ) as decided
      from folders f
      left join files s on s.folder_id = f.id
     group by f.id, f.parent_id, f.status, (f.props->>'expected_files')::bigint
),
incomplete as (
    -- Seed: a folder short of its own denominator. `expected is null` means
    -- NEVER WALKED, which is incomplete — and is deliberately distinct from
    -- expected = 0 (walked, holds no indexable files), which is complete.
    -- Collapsing the two would declare every unvisited folder finished.
    select id, parent_id
      from local
     where expected is null
        or decided < expected
    union
    -- Propagate UPWARD: a folder containing an incomplete descendant is itself
    -- incomplete. The recursion is over TREE STRUCTURE (parent_id), never over
    -- derived status — which is what lets one pass settle the whole hierarchy.
    -- A view that read `folders.status` to decide `folders.status` would be
    -- self-referential and need a fixpoint loop to converge.
    select l.id, l.parent_id
      from local l
      join incomplete i on i.parent_id = l.id
)
select l.id
     , l.parent_id
     , l.expected
     , l.decided
     , (i.id is null)                     as subtree_complete
     , l.status                           as stored_status
     , (l.status = 'indexed') is distinct from (i.id is null) as drifted
  from local l
  left join incomplete i on i.id = l.id;

comment on view folder_completeness is
'Is a folder fully indexed, derived from persisted per-file facts rather than from the task queue.

Folder status is otherwise set by the queue reaching DetectCommunities, which is not a dependable signal: the daily analyzer enqueues that task UNBLOCKED, so it can run against a partially-indexed folder. This view trusts only what is on disk in the database.

A folder is complete when every file it owns has reached a verdict AND every folder beneath it is complete. `expected` is the denominator, recorded by the walk — counting only the files rows that EXIST is vacuous, because a walk that died at file 40 of 100 leaves 40 decided rows and 60 with no row at all.

A verdict is `parsed_at is not null or skip_reason is not null`. It is NOT `indexed_at`: that column is NOT NULL DEFAULT now(), so testing it is a tautology that counts every walked file as decided and makes the whole view report 100% before any parsing has run.

`drifted` compares the stored status against the derived answer, so disagreement is directly queryable instead of being inferred from a suspiciously short result.';
