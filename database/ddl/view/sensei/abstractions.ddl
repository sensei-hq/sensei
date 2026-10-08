set search_path to sensei, extensions;

create or replace view abstractions as
select n.id
     , n.folder_id
     , n.project_id
     , n.project
     , n.fqn
     , n.name
     , n.kind::text        as declared_kind
     , n.language
     , n.file_path
     , n.line_start
     , n.is_exported
     , n.locality
     , split_part(n.fqn, '·', 2) as component
     , count(e.id)                                   as implementers
     , count(e.id) filter (where e.kind = 'extends')    as extended_by
     , count(e.id) filter (where e.kind = 'implements') as implemented_by
     , count(distinct src.folder_id)                 as implementing_folders
     -- APPENDED, not inserted beside `project`: `create or replace view`
     -- accepts new columns only at the end.
     --
     -- Carried through because the scalar `project` above is now NULL whenever
     -- the node's repository serves more than one project — `graph_nodes`
     -- refuses to pick one of two. Without these, 1,341 of 3,696 rows (36%)
     -- lost their project outright: policy-management 1,267 -> 0, mycm.net
     -- 43 -> 0, employee-portal 28 -> 0, kavach 3 -> 0. Filter with
     -- `= any(projects)`, never `project =`, or those rows vanish silently.
     , n.project_ids
     , n.projects
  from sensei.graph_nodes n
  join sensei.edges       e   on e.target_id = n.id and e.kind in ('implements', 'extends')
  join sensei.nodes       src on src.id      = e.source_id
 where n.fqn is not null
 group by n.id, n.folder_id, n.project_id, n.project, n.project_ids, n.projects, n.fqn, n.name, n.kind
        , n.language, n.file_path, n.line_start, n.is_exported, n.locality;

comment on view abstractions is
'A type something implements or extends — the graph''s answer to "what is an
abstraction here", and the input to Martin''s A (abstractness).

DERIVED FROM THE EDGES, NEVER FROM A KIND NAME, and that is the whole point.
The obvious rule — `kind = ''interface''` — is wrong because the keyword means
two different things. Measured on the live graph: rust records a `trait` as
`interface` and 21 of 21 have an implementer; TypeScript has 606 interfaces
with ONE between them, because a TS interface is usually a data shape
(`SessionRow`, `LogRow`, `ImpactBuckets`). Counting those as abstractions put
three TypeScript components at 90-100% abstract, which is nonsense.

Asking the edges instead is language-agnostic, self-correcting for TypeScript''s
overloaded keyword, and needs no per-language `is_abstract` flag that a walk
could get wrong. Verified: `LanguageAdapter` reports 12 implementers, which is
exactly the number of adapters its registry holds.

WHAT IT UNDERCOUNTS, said plainly: a contract declared and never implemented in
the indexed scope. Measured at ZERO for rust in this repository, and it is a
bounded, nameable gap rather than a silent one — the type is still a node, it
simply is not in this view.

Rests on `implements` and `extends`, the two best-resolved edge kinds in the
graph (59.2% and 71.6% overall; 92.4% and 97.9% on files indexed since the
cutover). A seam whose implementers have not resolved is under-reported, never
over-reported: an unresolved edge has no `target_id` and so joins nothing.

LOCALITY IS CARRIED, NOT FILTERED. Measured on this repository: 32 of the 50
abstractions are ours and 18 are std/library traits we implement — `Error`,
`Sync`, `Send`, `Display`. Both are true and they answer different questions.
Martin''s A wants the first; "what does this codebase conform to" wants the
second. Dropping either here would make one of those unanswerable, so the
consumer chooses with a WHERE clause.

Common queries:
  -- the polymorphic seams of a project, widest first
  SELECT name, component, implementers FROM abstractions WHERE ''sensei'' = any(projects) ORDER BY implementers DESC
  -- Martin''s A per component. OURS only: a std trait we implement is not an
  -- abstraction this codebase offers.
  SELECT component, count(*) FROM abstractions WHERE ''sensei'' = any(projects) AND locality = ''internal'' GROUP BY component
  -- a seam implemented across folder boundaries is an integration point
  SELECT name, implementing_folders FROM abstractions WHERE implementing_folders > 1';

comment on column abstractions.implementers is
'How many types implement or extend this one. The count a class diagram labels a seam with, and the reason this row exists at all — a type with none is not in this view.';
comment on column abstractions.component is
'The fqn''s package segment — the unit Martin''s Ca/Ce/I are computed over. For SQL nodes this is the schema, which is the same idea one level down.';
comment on column abstractions.implementing_folders is
'Distinct folders the implementers live in. More than one means the seam is an INTEGRATION point rather than a local convenience, which is what makes it worth naming on a diagram.';
