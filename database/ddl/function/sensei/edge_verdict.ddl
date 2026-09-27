set search_path to sensei, extensions;

-- The one edge-level verdict, reduced from the per-use verdicts an edge carries.
--
-- An edge is a RELATIONSHIP and its identity is `(folder, source, target, kind)`,
-- so one row aggregates every use site of that relationship — two calls to one
-- function from one function are one row. The indexer records a verdict on each
-- USE, nested under the file that made it:
--
--   props.occurrences["src/a.rs"][0].rung     -- when the ladder placed it
--   props.occurrences["src/a.rs"][0].reason   -- when it could not
--
-- The file level is not decoration: two files can produce one edge (two
-- declarations minting one fqn are one node), and `props || EXCLUDED.props`
-- REPLACES a key rather than appending, so an unkeyed array meant the second
-- file's write erased the first's list. See `indexer/persist.rs`.
--
-- LOWEST PRECEDENCE WINS, and it is the same rule on both sides.
-- `reason_codes.precedence` is climb order for a rung and severity order for a
-- reason, so the strongest proof and the most serious fault both sort first. An
-- edge whose uses are `plumbing` (a deliberate refusal, 90) and
-- `receiver_type_unknown` (a fault, 30) reports the FAULT — a real gap must not
-- be hidden by a filter that happens to share the edge.
--
-- ONE OWNER, because there are three callers and a second copy of a reduction is
-- how two of them come to disagree: `merge_edge_occurrences` and
-- `drop_edge_occurrences` both re-derive the stored column after changing the
-- list, and `call_graph` falls back to this for rows written before the columns
-- existed.
--
-- Measured before it existed: `resolved_via` was NULL on all 1,640,215 placed
-- edges and `unresolved_reason` NULL on all 2,428,016 missed ones, because both
-- were read as `props->>'rung'` at the TOP level and no writer has ever put them
-- there. Neither a match nor a miss could be classified, in any language.
--
-- `field` is 'rung' or 'reason'; `code_domain` the matching `reason_codes.domain`
-- ('code_graph_rung' or 'code_graph'). Two arguments rather than two functions
-- because the reduction is identical and only the vocabulary differs — splitting
-- it would be the second copy this exists to prevent.
--
-- STABLE, not IMMUTABLE: it reads `reason_codes`.
drop function if exists edge_verdict cascade;

create or replace function edge_verdict(
  occurrences jsonb,
  code_domain text,
  field       text
)
returns text
language sql
stable
as $$
  -- `jsonb_path_query` rather than `jsonb_each` + `jsonb_array_elements`:
  -- jsonpath is LAX, so a props shape that does not match yields no rows instead
  -- of raising "cannot extract elements from a scalar". One malformed edge must
  -- not fail every query over its folder.
  select rc.code
    from jsonb_path_query(
           coalesce(occurrences, '{}'::jsonb),
           ('$.*[*].' || field)::jsonpath) v
    join sensei.reason_codes rc
      on rc.domain = code_domain
     and rc.code   = v #>> '{}'
   order by rc.precedence
   limit 1
$$;

comment on function edge_verdict(jsonb, text, text) is
'The one edge-level verdict reduced from an edge''s per-use verdicts: the
lowest-precedence code of `code_domain` found at `props.occurrences.*[*].<field>`.
Lowest precedence wins, so a fault outranks a refusal sharing the edge. Called by
merge_edge_occurrences, drop_edge_occurrences and the call_graph view.';
