set search_path to sensei, extensions;

create or replace view graph_placement as
with e as (
  select f.project_id
       , coalesce(p.name, f.name)                     as project
       , ed.kind::text                                as edge_kind
       , ed.resolved_via
       , ed.unresolved_reason
    from sensei.edges     ed
    join sensei.folders   f on f.id = ed.folder_id
    left join sensei.projects p on p.id = f.project_id
)
select e.project
     , e.project_id
     , e.edge_kind
     , case
         when e.resolved_via      is not null then 'placed'
         when e.unresolved_reason is not null then 'missed'
         else                                      'no verdict'
       end                                            as outcome
     , coalesce(e.resolved_via, e.unresolved_reason)  as code
     , rc.kind::text                                  as code_kind
     , rc.precedence
     , rc.summary
     , count(*)                                       as edges
     , round(100.0 * count(*)
             / sum(count(*)) over (partition by e.project, e.edge_kind), 2) as pct_of_kind
     , round(100.0 * count(*)
             / nullif(sum(count(*)) over (partition by e.project), 0), 2)   as pct_of_project
  from e
  left join sensei.reason_codes rc
    on rc.code = coalesce(e.resolved_via, e.unresolved_reason)
   and rc.domain = case when e.resolved_via is not null
                        then 'code_graph_rung' else 'code_graph' end
 group by e.project, e.project_id, e.edge_kind, e.resolved_via, e.unresolved_reason
        , rc.kind, rc.precedence, rc.summary;

comment on view graph_placement is
'How every edge in the graph turned out, decomposed — one row per
(project, edge_kind, verdict) with the share it accounts for.

THE DECOMPOSITION IS THE POINT, not the headline. A single "N% resolved" cannot
tell `declared_here` — the file declares the target itself — from
`in_the_prelude`, where the target is a guess among identically-named things,
and both are "resolved". Nor can it tell a FAULT someone can close from a
REFUSAL that is correct. `code_kind` carries that split straight off
`reason_codes`, so a reader can sum the faults without knowing the vocabulary.

THREE OUTCOMES, NOT TWO. `no verdict` is its own row and must stay visible: an
edge that is neither placed nor explained is not a miss, it is a question
NOBODY ASKED, and folding it into "unresolved" would report a resolver that
tried and failed when nothing tried. Measured at 240,439 rows when this was
written, which is why it earns a name.

PERCENTAGES ARE COMPUTED HERE so every consumer divides by the same
denominator. `pct_of_kind` is the honest one for comparing resolution quality —
`imports` and `calls` have wildly different populations — and `pct_of_project`
is what a summary bar renders.

LEFT JOIN on reason_codes, deliberately. A code with no registry row still gets
its count, with `code_kind` and `summary` null: an unregistered verdict is a
gap in the VOCABULARY, and dropping the row would hide the very thing that needs
fixing. (`edge_verdict` cannot be so forgiving — it reduces BY precedence, so an
unregistered code there is silently lost. That asymmetry is why a new rung must
be seeded before it is emitted.)

Common queries:
  -- the headline, per edge kind
  SELECT edge_kind, outcome, sum(edges), sum(pct_of_kind) FROM graph_placement WHERE project = ''sensei'' GROUP BY 1, 2
  -- which rung is doing the work
  SELECT code, edges, pct_of_kind FROM graph_placement WHERE project = ''sensei'' AND outcome = ''placed'' ORDER BY edges DESC
  -- the faults, which are the only misses worth chasing
  SELECT code, summary, sum(edges) FROM graph_placement WHERE code_kind = ''fault'' GROUP BY 1, 2 ORDER BY 3 DESC';

comment on column graph_placement.outcome is
'placed | missed | no verdict. The third is not a kind of miss — it is an edge no resolver ever reached a conclusion about, and it disappears the moment it is folded into the second.';
comment on column graph_placement.code_kind is
'From reason_codes: normal | refusal | fault. A refusal is correct and needs no work; a fault is a gap someone can close. Null means the code is not registered, which is itself a finding.';
comment on column graph_placement.pct_of_kind is
'Share of this edge kind in this project. The denominator to compare resolution quality across kinds, because populations differ by orders of magnitude.';
