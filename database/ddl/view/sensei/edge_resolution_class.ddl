set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.edge_resolution_class AS
SELECT e.id
     , e.folder_id
     , e.kind
     , CASE
         -- NOT the same as "correctly resolved": 152,293 resolved edges (46.7%)
         -- point at a STUB (`file_path IS NULL`), 96,646 of them at a
         -- non-lib internal stub. A stub is a placeholder awaiting enrichment,
         -- so this bucket mixes real links with pending ones.
         WHEN e.target_id IS NOT NULL THEN 'resolved'
         -- Exactly one node in this folder bears the name. NAMED FOR WHAT IT IS
         -- — a name collision of degree one — NOT for what it means. See the
         -- comment below: this is NOT a defect queue.
         WHEN (SELECT count(*) FROM sensei.nodes n
                WHERE n.folder_id = e.folder_id
                  AND n.name = e.target_name
                  AND n.file_path IS NOT NULL) = 1 THEN 'name-collision-1'
         WHEN (SELECT count(*) FROM sensei.nodes n
                WHERE n.folder_id = e.folder_id
                  AND n.name = e.target_name
                  AND n.file_path IS NOT NULL) > 1 THEN 'name-collision-n'
         ELSE 'no-local-name'
       END AS resolution_class
  FROM sensei.edges e;

comment on view edge_resolution_class is
'Partitions edges by whether any LOCAL NODE SHARES THE TARGET NAME. That is all
it measures. Read the warnings before using any number from it.

WHY THE CLASSES ARE NAMED THIS WAY. An earlier version called these
`unambiguous-miss` / `ambiguous` / `absent` and presented the first as "THE
DEFECT CLASS" of 27,495 edges to drive to zero. Adversarial review refuted that
on three counts, each verified:

1. THE ONLY MECHANISM THAT DRIVES `name-collision-1` TO ZERO IS THE RESOLVER
   THIS CODEBASE ALREADY REJECTED. The predicate here is
   `sole_definition_id_by_name` (pg_store/graph.rs) minus its kind filter, and
   tasks/handlers/process.rs documents that exact resolver as refused because
   "a miss would resolve confidently WRONG". Optimising this number means
   reintroducing bare-name matching, which the CLAUDE.md no-fabrication rule
   forbids.

2. THE POPULATION IS MOSTLY NOT DEFECTS. The head of `name-collision-1` for
   calls is `json` 1,600 · `path` 483 · `all` 455 · `join` 443 · `text` 402 ·
   `find` 379 — external accessor methods (`request.json()`, `dir.path()`,
   `.iter().find()`) that happen to share a name with one local symbol. 4,325
   of 27,510 (15.7%) name something the FQN resolver ITSELF already resolved
   externally in the same folder; for imports that is 381 of 496 (76.8%). A
   12-edge sample ran ~42% false positive. 200 have a sole candidate that is
   not callable at all (188 modules, 9 classes).

3. IT IS NOT AN UPPER BOUND, IN EITHER DIRECTION. Defects sit OUTSIDE it:
   3,982 `name-collision-n` edges have exactly ONE same-named candidate in the
   SAME FILE as the source (1,602 excluding self-recursion), which ordinary
   lexical scope resolves without guessing. Verified case:
   `GradleManifestAdapter.detect_workspace_members` at adapters/manifest/gradle.rs
   is called at :415 with its only in-file definition at :82, yet lands in
   `name-collision-n` because 7 manifest adapters define that method name.

THE `file_path IS NOT NULL` CANDIDATE CLAUSE IS A JUDGEMENT CALL, NOT A FACT.
Removing it moves 59,141 edges and takes the degree-one count from 27,501 to
53,126 (+93%). Stubs are excluded on the grounds that they are placeholders,
but stubs ARE the target of 46.7% of all resolved edges — and at least one
internal-KINDED stub (`rust·senseid·tasks::handlers::metrics·QueryAs·bind`) is
actually external sqlx, so neither candidate set is trustworthy. Settle it by
sampling internal-kinded stubs against the filesystem before relying on either
figure.

WHAT THIS VIEW IS GOOD FOR: `no-local-name` is the solid finding. 109,944 of
110,785 unresolved imports (99.2%) name nothing local, which is why
externals-as-lib_symbol is the right treatment for imports rather than better
local resolution. Use it for that, and do not use the other classes to size
work until a real defect definition exists.';
