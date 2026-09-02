set search_path to sensei, extensions;

-- Every node with its locality and its parent — the dimension `call_graph` lacks.
--
-- ## Why this exists
--
-- Until now the only way to ask "is this external?" was to ask "did the edge fail
-- to resolve?" — the proxy in the since-deleted `build_connections`. It was wrong in BOTH
-- directions. Measured 2026-09-01 on the live DB: of the 1,040 entries it wrote
-- into `folders.props.libs` for the `sensei` folder, **791 were this repo's own
-- code** — `crate::log_collector::LogCollector`, `./WizardRail.svelte`, `$lib/nav`.
-- rokkit read 807 of 890 (91%), OmniRoute 4,085 of 5,468. And it would have lost
-- every genuine dependency the moment resolution started working, because
-- `target_id` and `target_name` are mutually exclusive across all 715,757 edges —
-- resolving an edge ERASES the name the count was reading.
--
-- Java folders read 0 false positives purely by accident: Java has no relative
-- import form, so every Java import is an absolute FQN.
--
-- Locality is a property of the NODE, not of an edge's resolution state. Asking
-- the node makes both failure modes structurally impossible.
--
-- ## This is a projection, not a second copy of a rule
--
-- `languages::import_target::classify_import` parses a specifier string (`./x`,
-- `$lib/x`, `node:fs`, `java.util.List`) and exercises judgment; it stays in Rust
-- with ONE owner, because a SQL copy of a judgement rule is how the scan exclusion
-- resolver came to gate the watcher while pruning nothing (see
-- `import_target_counts`).
--
-- This view exercises no judgement. It reports the decision the WRITER already
-- recorded: `upsert_lib_symbol` sets `kind = 'lib_symbol'` for a dependency's
-- symbol, and a definition in a local file gets a `file_path`. Reading those two
-- columns cannot drift from the rule that set them, because it IS the rule's
-- output.
--
-- ## Three-valued, deliberately
--
-- A boolean would bin the 85,530 nodes with neither a `file_path` nor a
-- `lib_symbol` kind as external, reproducing exactly the false positives above.
-- Those are the unresolved reference stubs (84,396 of them `kind='function'`), and
-- `unknown` makes them countable — which is what turns the invariant "stub count
-- → 0" into a one-line query instead of a research project.
--
-- `nodes.resolved` is deliberately NOT the locality signal: 140,051 `section` rows
-- are `resolved=false` while sitting in real files, so that column answers "did
-- FQN enrichment run", not "where does this live". It is projected for filtering,
-- never for classification.
--
-- ## Hierarchy
--
-- `parent_id` already carries containment (296,744 of 430,977 rows), so a grouping
-- view needs no `contains` edge kind — parent for the bubbles, edges for the
-- lines. `parent_name`/`parent_kind` are surfaced so callers group without a
-- self-join, and are NULL for a top-level node rather than a placeholder.
create or replace view graph_nodes as
select n.id
     , n.folder_id
     , f.name         as folder
     , f.branch       as branch
     , f.project_id
     , p.name         as project
     , n.kind::text   as kind
     , n.name
     , n.fqn
     , n.language
     , n.file_path
     , n.line_start
     , n.resolved
     , n.is_exported
     , n.is_test
     , n.community_id
     , case
         -- The writer records a dependency's symbol and its package container
         -- with these kinds; both are external by construction.
         when n.kind in ('lib_symbol', 'lib_package') then 'external'
         -- A local file is the definition of internal.
         when n.file_path is not null                 then 'internal'
         -- Neither: an unresolved reference stub. Saying "external" here is the
         -- bug this view replaces.
         else                                              'unknown'
       end            as locality
     , n.parent_id
     , par.name       as parent_name
     , par.kind::text as parent_kind
  from nodes         n
  join folders       f
    on f.id          = n.folder_id
  left join projects p
    on p.id          = f.project_id
  left join nodes    par
    on par.id        = n.parent_id;

comment on view graph_nodes is
'Every node with its LOCALITY (internal | external | unknown) and its parent.

locality is read from what the writer recorded — kind in (lib_symbol, lib_package)
=> external; a non-null file_path => internal; neither => unknown (an unresolved
reference stub). It is NOT derived from an edge failing to resolve, which is the
proxy that reported 791 of 1,040 of this repo''s own modules as dependencies.
It is NOT derived from nodes.resolved either: 140,051 section rows are
resolved=false while sitting in real files.

Three-valued on purpose: a boolean bins the 85,530 stubs as external. `unknown`
makes them countable, so "stub count -> 0" is a query.

parent_id/parent_name/parent_kind carry containment, so grouping needs no
`contains` edge kind: parent for the nesting, edges for the connections.

Common queries:
  -- dependency count, correct where folders.props.libs was not
  SELECT count(DISTINCT n.name) FROM sensei.edges e
    JOIN sensei.graph_nodes n ON n.id = e.target_id
   WHERE e.folder_id = ''...'' AND n.locality = ''external''

  -- the invariant the identity fix has to drive to zero
  SELECT folder, count(*) FROM sensei.graph_nodes
   WHERE locality = ''unknown'' GROUP BY folder ORDER BY 2 DESC

  -- patterns vs graph: pick the relation kinds, group by the hierarchy
  SELECT n.parent_name, n.kind, count(*) FROM sensei.graph_nodes n
   WHERE n.project = ''sensei'' AND n.locality = ''internal'' GROUP BY 1, 2';

comment on column graph_nodes.branch is 'The checked-out branch of this node''s folder. A real partition key, not a label: the design is one folder per checkout (develop vs main = two folders, one repository), already live for fitness/strategos/website — so filtering by branch separates genuine graphs without branch appearing in node identity.';
comment on column graph_nodes.locality is 'internal (has a local file) | external (lib_symbol/lib_package — the writer said so) | unknown (unresolved reference stub). Never inferred from an edge''s resolution state.';
comment on column graph_nodes.parent_name is 'Containing node''s name — NULL at top level, not a placeholder. Lets a caller group by container without a self-join.';
comment on column graph_nodes.resolved is 'Whether FQN enrichment ran. Projected for filtering; NOT a locality signal — 140,051 section rows are resolved=false inside real files.';
