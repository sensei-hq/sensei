set search_path to sensei, extensions;

-- How one project's edges turned out, decomposed by LANGUAGE as well as by kind
-- and verdict (#235).
--
-- `graph_placement` already decomposes by (project, edge_kind, verdict, reason
-- code) and already splits a reason into `normal` / `fault` / `refusal`. That
-- taxonomy is good and this does not replace it. What it lacks is the one axis
-- that makes a corpus-wide number READABLE: the language the edge was written
-- in.
--
-- Why that axis decides things. Measured 2026-10-06, `calls` places at 29.6%
-- corpus-wide. By language: csharp 15.5%, java 43%, c 26%. C# is dragging the
-- headline down, and C# is also the language with the worst indexing coverage
-- (52.3% of its tracked files carry a node, #237). Those are TWO different
-- defects stacking, and without this axis a change to either moves the same
-- number with nothing to say which one moved it.
--
-- A FUNCTION, not a view, for the same reason `component_zones` is: the join to
-- `nodes` for the source language costs 4m20s corpus-wide and the predicate has
-- to be structural rather than hoped-for. Even so this is an ANALYSIS query, not
-- a screen query — expect seconds to minutes depending on the project.
drop function if exists resolution_quality(uuid) cascade;

create or replace function resolution_quality(p_project_id uuid)
returns table (
  language       text
, edge_kind      text
, outcome        text
, code           text
, code_kind      text
, edges          bigint
, pct_of_lang_kind numeric
)
language sql
stable
-- Pinned rather than inherited: the daemon connects with no `sensei` on its
-- path and fully qualifies every statement.
set search_path = sensei, extensions
as $fn$
  with e as (
    select coalesce(sn.language, '(unknown)') as language
         , ed.kind::text                      as edge_kind
         , ed.resolved_via
         , ed.unresolved_reason
      from sensei.folder_projects fp
      cross join lateral (
        -- The LATERAL + fence `structure_edges` documents. Without it the
        -- planner reads all 3.3M edges instead of this project's slice.
        select e2.kind, e2.source_id, e2.resolved_via, e2.unresolved_reason
          from sensei.edges e2
         where e2.folder_id = fp.folder_id
         offset 0
      ) ed
      join sensei.nodes sn on sn.id = ed.source_id
     where fp.project_id = p_project_id
  )
  select e.language
       , e.edge_kind
       -- THREE outcomes, never two. An edge with neither verdict was not
       -- attempted, and folding it into "missed" reports a resolver that tried
       -- and failed when nothing tried. Measured 2026-10-06: 251,270 of 258,623
       -- import edges are in exactly that state, which made the "imports resolve
       -- at 2.6%" reading meaningless rather than bad (#242).
       , case
           when e.resolved_via      is not null then 'placed'
           when e.unresolved_reason is not null then 'missed'
           else                                      'no verdict'
         end                                             as outcome
       , coalesce(e.resolved_via, e.unresolved_reason)   as code
       , rc.kind::text                                   as code_kind
       , count(*)                                        as edges
       -- Denominator is the (language, kind) cell, which is the only comparison
       -- that means anything: `imports` and `calls` have populations two orders
       -- of magnitude apart, and so do C# and C.
       , round(100.0 * count(*)
               / sum(count(*)) over (partition by e.language, e.edge_kind), 2)
                                                         as pct_of_lang_kind
    from e
    left join sensei.reason_codes rc
      on rc.code = coalesce(e.resolved_via, e.unresolved_reason)
     and rc.domain = case when e.resolved_via is not null
                          then 'code_graph_rung' else 'code_graph' end
   group by e.language, e.edge_kind, e.resolved_via, e.unresolved_reason, rc.kind
$fn$;

comment on function resolution_quality(uuid) is
'Resolution decomposed by (language, edge kind, verdict, reason code) for one
project — the instrument that makes a change to the indexer ATTRIBUTABLE.

WHY THE LANGUAGE AXIS IS THE WHOLE POINT. A corpus-wide percentage mixes two
different defects. Measured 2026-10-06: `calls` places at 29.6% across the
corpus, but at 15.5% for csharp, 43% for java and 26% for c. C# is dragging the
headline down, and C# is separately the language whose tracked files are only
52.3% indexed. "Never indexed" and "indexed and the resolver missed" need
different fixes and different owners; one number cannot tell them apart, and
changing either without this breakdown means changing it blind.

LANGUAGE IS THE SOURCE NODE''S, not the target''s. The question this answers is
"how well does the adapter that wrote this edge resolve", so attribution follows
the file the edge was written in. An edge whose source node carries no language
is reported as `(unknown)` rather than dropped — a population that cannot be
attributed is a finding, not a rounding error.

THREE OUTCOMES, NEVER TWO, inherited from `graph_placement` and load-bearing. An
edge with neither `resolved_via` nor `unresolved_reason` was NOT ATTEMPTED.
Folding it into "missed" reports a resolver that tried and failed when nothing
tried. Measured: 251,270 of 258,623 import edges are exactly that, which is why
"imports resolve at 2.6%" was a meaningless number rather than a bad one (#242) —
it divided by a population nothing had asked about.

`code_kind` CARRIES THE FAULT/REFUSAL SPLIT straight off `reason_codes`, so a
reader can sum what is closeable without learning the vocabulary: `fault` is a
gap someone can close, `refusal` is structurally unanswerable (dynamic dispatch,
macro expansion, the external boundary), `normal` is a placement that went well.

THIS DOES NOT REPLACE `graph_placement`. That view answers the same question
corpus-wide without the language join, which is far cheaper, and it computes
`pct_of_project` for a summary bar. Use it for a screen; use this for a
decision about where to spend indexer effort.

AN ANALYSIS QUERY, NOT A SCREEN QUERY. The join to `nodes` for the source
language costs 4m20s corpus-wide; per project it is seconds to minutes. Nothing
on a latency budget should call it.

Common queries:
  -- where is the resolver actually losing, worst cell first
  SELECT language, edge_kind, sum(edges) FILTER (WHERE outcome = ''missed'') AS missed
    FROM resolution_quality($1) GROUP BY 1, 2 ORDER BY 3 DESC
  -- how much of the loss is closeable rather than structural
  SELECT language, code_kind, sum(edges) FROM resolution_quality($1)
   WHERE outcome = ''missed'' GROUP BY 1, 2
  -- anything nobody asked about
  SELECT language, edge_kind, sum(edges) FROM resolution_quality($1)
   WHERE outcome = ''no verdict'' GROUP BY 1, 2 ORDER BY 3 DESC';
