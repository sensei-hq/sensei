set search_path to sensei, extensions;

-- A FUNCTION, not a view, and the reason is measured (#223).
--
-- The computation has four independent halves — the module universe, afferent
-- coupling, efferent coupling, and coupling onto library surface — and a module
-- with no coupling must survive all of them, so the universe LEFT JOINs the
-- other three. PostgreSQL does not form equivalence classes across an outer
-- join, so a caller's `WHERE project_id = $1` on a view cannot reach the
-- nullable sides: each would be computed for the WHOLE corpus. Measured
-- 2026-10-05 on project `sensei`, that shape had not returned after ten minutes.
--
-- A parameter makes the scope structural. Every branch below binds
-- `p_project_id` itself, so no plan shape and no future rewrite can lose it.
drop function if exists component_zones(uuid) cascade;

create or replace function component_zones(p_project_id uuid)
returns table (
  project        text
, project_id     uuid
, component      text
, types          bigint
, abstract_types bigint
, ca             bigint
, ce             bigint
, abstractness   double precision
, instability    double precision
, distance       double precision
, zone           text
)
language sql
stable
-- Pinned, not inherited. The daemon connects with no `sensei` on its path and
-- fully qualifies every statement, so an unqualified reference inside this body
-- would resolve only by luck — `repo_anchor_for` pins its path for the same
-- reason. The body also qualifies, so neither alone is load-bearing.
set search_path = sensei, extensions
as $fn$
with me as (
  -- The module-to-module edge set, ONCE. Read twice — for Ca and for Ce — and
  -- `module_edges` is expensive enough that reading it twice is the difference
  -- between a query and a hang.
  select e.project_id, e.source_module, e.target_module
    from sensei.module_edges e
   where e.project_id = p_project_id
     and e.source_module <> e.target_module
   group by 1, 2, 3
),
afferent as (
  -- DISTINCT PARTNERS, not occurrences: Martin counts how many things depend on
  -- a component, and calling one of them nine times is still one dependency.
  select m.project_id, m.target_module as component, count(distinct m.source_module) as ca
    from me m group by 1, 2
),
efferent as (
  select m.project_id, m.source_module as component, count(distinct m.target_module) as ce
    from me m group by 1, 2
),
external as (
  -- Efferent coupling onto LIBRARY surface, counted once per package. A module
  -- whose only dependencies are libraries is not stable, and internal-only
  -- coupling would say it was.
  select fp.project_id
       , sensei.module_of(split_part(sn.fqn, '·', 2), split_part(sn.fqn, '·', 3)) as component
       , count(distinct split_part(tn.fqn, '·', 2))                        as ce_external
    from sensei.folder_projects fp
    cross join lateral (
      -- The same LATERAL + fence `structure_edges` documents: without it the
      -- planner reads all of `edges` instead of this project's slice.
      select e2.source_id, e2.target_id
        from sensei.edges e2
       where e2.folder_id = fp.folder_id
         and e2.target_id is not null
       offset 0
    ) ed
    join sensei.nodes sn on sn.id = ed.source_id
    join sensei.nodes tn on tn.id = ed.target_id
   where fp.project_id = p_project_id
     and sn.file_id is not null
     and sn.fqn     is not null
     and tn.file_id is null
     and tn.fqn like 'lib·%'
     -- `$lib` is SvelteKit's alias for the package's OWN source, so counting it
     -- as an external dependency inflates Ce with the module's own code. `$app`
     -- is the framework runtime and genuinely is external, so it stays.
     and split_part(tn.fqn, '·', 2) <> '$lib'
   group by 1, 2
),
recognised as (
  -- THE SHARED NODE UNIVERSE. A module is one that `structure_graph` names —
  -- i.e. one that is the modal module of at least one FILE — which is exactly
  -- the node set the Structure, Layers and Cycles diagrams draw. Four screens
  -- disagreeing about what a module is would be worse than any of them being
  -- slightly wrong.
  --
  -- It is also what keeps #231 off this plot. An fqn''s third segment is the
  -- module for most adapters and a SYMBOL for some, so a top-level symbol mints
  -- a module-shaped name no file is modal for. Measured 2026-10-05 on project
  -- `sensei`: without this join the point set is 391 "modules" against a real
  -- 151, and the worst-distance list is led by a C struct and three protocol
  -- payload types rather than by anything a reader could act on.
  select distinct sensei.module_of(g.package, g.module) as component
    from sensei.structure_graph g
   where g.project_id = p_project_id
),
universe as (
  -- Abstractness is counted per SYMBOL, matching how `module_edges` attributes
  -- a dependency, so the two halves of each point are measured the same way.
  -- A module with no coupling at all still has an abstractness, and a module
  -- that declares no type still has a position on the instability axis.
  select fp.project_id
       , fp.project
       , sensei.module_of(split_part(n.fqn, '·', 2), split_part(n.fqn, '·', 3)) as component
       , count(*) filter (
           where n.kind in ('class', 'interface', 'struct', 'enum', 'type', 'trait')
         )                                                               as types
       , count(*) filter (
           where n.kind in ('class', 'interface', 'struct', 'enum', 'type', 'trait')
             and exists (select 1 from sensei.edges ab
                          where ab.target_id = n.id
                            and ab.kind in ('implements', 'extends'))
         )                                                               as abstract_types
    from sensei.folder_projects fp
    join sensei.nodes n on n.folder_id = fp.folder_id
   where fp.project_id = p_project_id
     and n.file_id is not null
     and n.fqn     is not null
     and split_part(n.fqn, '·', 2) <> ''
     and exists (select 1 from recognised r
                  where r.component = sensei.module_of(split_part(n.fqn, '·', 2),
                                                       split_part(n.fqn, '·', 3)))
   group by 1, 2, 3
),
scored as (
  -- LEFT JOINED, and that is why this is a function. A module with no coupling
  -- must survive — an inner join would delete the very rows `instability IS
  -- NULL` exists to report — but PostgreSQL will not push a predicate across an
  -- outer join, so as a VIEW each nullable side was computed for the whole
  -- corpus. Here every branch binds `p_project_id` itself, so the scope cannot
  -- be lost.
  --
  -- Correlated scalar subqueries were the first shape and are worse again:
  -- `external` costs a scan of the project's edges, re-run per module.
  select u.project_id
       , u.project
       , u.component
       , u.types
       , u.abstract_types
       , coalesce(af.ca, 0)::bigint                                     as ca
       , (coalesce(ef.ce, 0) + coalesce(x.ce_external, 0))::bigint      as ce
       -- NULL, NEVER ZERO, when the denominator is empty. See the function comment.
       , case when u.types > 0
              then u.abstract_types::double precision / u.types end     as abstractness
       , case when coalesce(af.ca, 0) + coalesce(ef.ce, 0) + coalesce(x.ce_external, 0) > 0
              then (coalesce(ef.ce, 0) + coalesce(x.ce_external, 0))::double precision
                 / (coalesce(af.ca, 0) + coalesce(ef.ce, 0) + coalesce(x.ce_external, 0))
              end                                                       as instability
    from universe u
    left join afferent af
           on af.project_id = u.project_id and af.component = u.component
    left join efferent ef
           on ef.project_id = u.project_id and ef.component = u.component
    left join external x
           on  x.project_id = u.project_id and  x.component = u.component
)
select s.project
     , s.project_id
     , s.component
     , s.types
     , s.abstract_types
     , s.ca
     , s.ce
     , s.abstractness
     , s.instability
     , abs(s.abstractness + s.instability - 1)                          as distance
     , case
         when s.abstractness is null or s.instability is null then null
         when abs(s.abstractness + s.instability - 1) > 0.4
           then case when s.abstractness + s.instability < 1 then 'pain' else 'useless' end
         when abs(s.abstractness + s.instability - 1) > 0.25 then 'risk'
         else 'main'
       end                                                              as zone
  from scored s
$fn$;

comment on function component_zones(uuid) is
'Robert C. Martin''s abstractness against instability, per module — the point set
behind the Zones diagram.

`A` is how much of a module is abstraction, `I` how much it depends outward
rather than being depended upon, and `D = |A + I − 1|` how far it sits from the
main sequence where the two balance. Far below it (`pain`) is concrete and widely
depended upon — rigid, expensive to change. Far above it (`useless`) is abstract
and depended upon by nothing.

NULL IS THE ANSWER WHEN THE DENOMINATOR IS EMPTY, and this is the one place this
view deliberately differs from the mockup. The mockup computes
`A = abstract / types`, which is NaN for a module declaring no type, and
`I = (Ca+Ce) ? Ce/(Ca+Ce) : 0`, which puts an isolated module at perfect
stability. Both are fabrications: a module that declares no type is not
"maximally concrete", and one nothing touches is not "maximally stable". Either
would land the point at a corner of the plot and invite a reading the data does
not support. Measured 2026-10-05 on project `sensei`: 50 of 165 modules that
appear in the edge set declare no type at all, so this is a third of the
population rather than an edge case. A consumer must OMIT a NULL point and SAY
how many it omitted.

ABSTRACTNESS IS DERIVED FROM THE EDGES, never from a keyword — a type is
abstract here if something `implements` or `extends` it. `sensei.abstractions`
owns that argument in full; the short version is that `kind = ''interface''`
means two different things (rust records a trait as `interface` and 21 of 21
have an implementer; TypeScript has 606 with one between them, because a TS
interface is usually a data shape). The denominator is every type-declaring
kind — class, interface, struct, enum, type, trait — and NOT `enum_variant`,
which is a value of a type rather than a type.

WHAT `A` UNDERCOUNTS, said plainly: a contract declared and never implemented in
the indexed scope is a type with no abstraction edge, so it lands in the
denominator and not the numerator. `abstractions` measures that gap at ZERO for
rust in this repository, but it is real for any language whose implementations
sit outside the index.

`Ca` AND `Ce` COUNT DISTINCT PARTNERS, not occurrences. Martin asks how many
things depend on a component; calling one of them nine times is one dependency.
A module''s dependency on ITSELF is excluded — it is cohesion, not coupling, and
counting it would make every module look one notch more unstable.

`Ce` INCLUDES LIBRARY PACKAGES, counted once each. A module whose only
dependencies are libraries has `ce_internal = 0`, and an internal-only `I` would
call it perfectly stable when it is the opposite. Measured: 825 module-library
pairs over 189 distinct packages for project `sensei`. `$lib` is excluded
because SvelteKit resolves it to the package''s own source; `$app` is kept
because the framework runtime genuinely is external. That exclusion is worth 8 of
the 825 pairs and changes no module''s count to zero — it is correctness, not
significance.

`Ca` COUNTS INTERNAL DEPENDENTS ONLY, because a library cannot depend on this
project.

THE GRAIN IS THE MODULE, matching the mockup (`module level`, and the formulas
at `…v8.dc.html:1925-1926`). Package grain is available by rolling up
`component`, but a project here is single digits of packages, which is too few
to read a main sequence off.

INHERITS `module_edges`'' COST AND ITS COVERAGE. Every caveat there applies:
placed edges only, project resolved through `folder_projects`, and an endpoint
that reads a module naming no file cannot place it.


Columns:
- `component` — The module, named by `module_of(package, module)` — the SAME identity `structure_graph`''s module rollup and `module_edges` use, so a point here and an edge there name one unit.
- `types` — Type-declaring symbols in the module: class, interface, struct, enum, type, trait. The denominator of `A`, and the circle''s area on the diagram.
- `abstract_types` — Of those, the ones something `implements` or `extends`. Derived from the edges, never from a keyword — see `sensei.abstractions`.
- `ca` — Afferent coupling: DISTINCT internal modules that depend on this one. Libraries cannot, so they are not counted here.
- `ce` — Efferent coupling: DISTINCT internal modules this one depends on, PLUS distinct library packages it uses. Excluding libraries would call a module that only uses them perfectly stable.
- `abstractness` — `abstract_types / types`, or NULL when the module declares no type. NEVER 0 on an empty denominator — that would read as "maximally concrete".
- `instability` — `ce / (ca + ce)`, or NULL when the module has no coupling at all. NEVER 0 on an empty denominator — that would read as "maximally stable".
- `distance` — `|A + I − 1|`, the distance from the main sequence. NULL when either input is.
- `zone` — pain | useless | risk | main, from `distance` at the mockup''s thresholds (0.4 and 0.25). NULL when `distance` is — a point with no position is in no zone.

Common queries:
  -- the point set for one project''s Zones diagram
  SELECT * FROM component_zones($1) WHERE distance IS NOT NULL ORDER BY distance DESC
  -- how many points cannot be placed, and why
  SELECT count(*) FILTER (WHERE abstractness IS NULL) AS no_types
       , count(*) FILTER (WHERE instability IS NULL)  AS no_coupling
    FROM component_zones($1)
  -- the worst offenders, worst first
  SELECT component, zone, round(distance::numeric, 2) FROM component_zones($1) WHERE zone IN (''pain'', ''useless'') ORDER BY distance DESC LIMIT 20';

