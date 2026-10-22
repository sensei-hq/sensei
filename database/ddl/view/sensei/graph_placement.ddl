set search_path to sensei, extensions;

create or replace view graph_placement as
with e as (
  select fp.workspace_id
       , fp.workspace
       , ed.kind::text                                as edge_kind
       , ed.resolved_via
       , ed.unresolved_reason
    from sensei.edges           ed
    join sensei.folder_workspaces fp on fp.folder_id = ed.folder_id
)
select e.workspace
     , e.workspace_id
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
             / sum(count(*)) over (partition by e.workspace, e.edge_kind), 2) as pct_of_kind
     , round(100.0 * count(*)
             / nullif(sum(count(*)) over (partition by e.workspace), 0), 2)   as pct_of_workspace
  from e
  left join sensei.reason_codes rc
    on rc.code = coalesce(e.resolved_via, e.unresolved_reason)
   and rc.domain = case when e.resolved_via is not null
                        then 'code_graph_rung' else 'code_graph' end
 group by e.workspace, e.workspace_id, e.edge_kind, e.resolved_via, e.unresolved_reason
        , rc.kind, rc.precedence, rc.summary;

comment on view graph_placement is
'How every edge in the graph turned out, decomposed — one row per
(workspace, edge_kind, verdict) with the share it accounts for.

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
`imports` and `calls` have wildly different populations — and `pct_of_workspace`
is what a summary bar renders.

LEFT JOIN on reason_codes, deliberately. A code with no registry row still gets
its count, with `code_kind` and `summary` null: an unregistered verdict is a
gap in the VOCABULARY, and dropping the row would hide the very thing that needs
fixing. (`edge_verdict` cannot be so forgiving — it reduces BY precedence, so an
unregistered code there is silently lost. That asymmetry is why a new rung must
be seeded before it is emitted.)

MEMBERSHIP COMES FROM `folder_workspaces`, NEVER FROM `folders.workspace_id`. That
column was settable independently on every folder, so a single repository''s
folders could disagree about which workspace they were in — and did. The path is
now edge -> folder -> repository -> `repositories_in_workspaces`, which is two indexed
joins and no function scan, so a repository''s edges cannot be split between
workspaces by accident. Measured when this was changed: the `kavach` repository''s
edges were split 5,347 / 2,834 between two workspaces and are now 8,181 under each.

WHICH MAKES THIS VIEW MULTI-VALUED, DELIBERATELY. A repository is keyed on its
REMOTE, and when this was written 8 repositories served two workspaces each,
between them holding 436,683 edges. Those edges are now counted under BOTH
workspaces. So `sum(edges)` over the WHOLE view (3,615,094 when written) is larger
than `count(*)` on `sensei.edges` (3,178,411) by exactly those 436,683 — SUM
WITHIN A WORKSPACE, NEVER ACROSS ONE.

That is not double counting, and the key is why: `repositories_in_workspaces` is keyed
on (workspace_id, repository_id) and a folder carries exactly one `repository_id`,
so a folder yields exactly ONE row per distinct workspace. No
(workspace, edge_kind, verdict) row can count the same edge twice, which is why
the GROUP BY here needs no DISTINCT and every percentage — already partitioned
by workspace — is unchanged by the multiplicity. Verified by counting one shared
workspace''s edges with and without `count(distinct edges.id)`: identical.

NO FOLDER-NAME FALLBACK ANY MORE. The old `coalesce(p.name, f.name)` labelled a
folder that had no workspace with the FOLDER''S OWN NAME, which in a result set is
indistinguishable from a real workspace. An edge whose folder sits under no
tracked repository now produces no row at all, which is what `folder_workspaces`
promises. Measured at 0 such edges when this was written, so nothing is lost
today — but the next one will be plainly absent rather than disguised.

Common queries:
  -- the headline, per edge kind
  SELECT edge_kind, outcome, sum(edges), sum(pct_of_kind) FROM graph_placement WHERE workspace = ''sensei'' GROUP BY 1, 2
  -- which rung is doing the work
  SELECT code, edges, pct_of_kind FROM graph_placement WHERE workspace = ''sensei'' AND outcome = ''placed'' ORDER BY edges DESC
  -- the faults, which are the only misses worth chasing
  SELECT code, summary, sum(edges) FROM graph_placement WHERE code_kind = ''fault'' GROUP BY 1, 2 ORDER BY 3 DESC';

comment on column graph_placement.outcome is
'placed | missed | no verdict. The third is not a kind of miss — it is an edge no resolver ever reached a conclusion about, and it disappears the moment it is folded into the second.';
comment on column graph_placement.code_kind is
'From reason_codes: normal | refusal | fault. A refusal is correct and needs no work; a fault is a gap someone can close. Null means the code is not registered, which is itself a finding.';
comment on column graph_placement.pct_of_kind is
'Share of this edge kind in this workspace. The denominator to compare resolution quality across kinds, because populations differ by orders of magnitude.';
comment on column graph_placement.workspace is
'From repositories_in_workspaces via folder_workspaces, not from folders.workspace_id. A repository serving two workspaces contributes its edges to BOTH, so always filter or group by this column — a total taken across workspaces over-counts the shared repositories.';
comment on column graph_placement.edges is
'Edges in this (workspace, edge_kind, verdict) bucket. Each edge is counted once per workspace its repository serves, so summing within one workspace is exact and summing across all of them is not.';
