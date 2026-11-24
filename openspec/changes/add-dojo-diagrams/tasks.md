# Tasks: add-dojo-diagrams (#268)

TDD: red test first for each item.

- [x] 1. Reads: `graph-reads.ts` (nodes/edges/files/schema per repo+branch,
      read-time resolution, external fallback); route specs with a fake
      client; DB tests proving a non-member reads nothing.
- [x] 2. Port: dōjō `CodeMapPort` (L0/L1/L2/zones from graph rows;
      cochange `commits: 0`; complexity `ok: false`); port specs over
      fixture rows. (Layering + L0/L1 math ported from the daemon and
      diffed byte/semantically equal against it; modules are folders,
      the grain the synced rows support.)
- [x] 3. Screens: repo diagram route + branch picker (`is_default`
      default) + lens switcher on `CodeMap`; team scoping for the
      world view. (Route: org/[slug]/repositories/[id]/diagrams/[lens];
      tenant-scoped navigation, `can_read_repository` the read gate;
      world/neighbourhood ride L0 + focus, no separate aggregate.)
- [x] 4. Tracked branches: authority-gated add/remove screen through
      `set_graph_branch`; the picker reflects the change. (Manager on
      the diagrams page behind `configurable_by_me`; two-step untrack;
      untracking the viewed branch moves to the tracked default.)
- [x] 5. Schema view: ER canvas over `graph_schema_*`. (Assemble
      ported byte-exact vs the daemon; dangling refs dropped like its
      inner join; enums named without values — values are not synced;
      schema is a lens on the repo diagrams route.)
- [x] 6. Gates green, commit, push. (Dojo 2527 green,
      svelte-check clean, vite build green, DB dojo 83 files green.)
