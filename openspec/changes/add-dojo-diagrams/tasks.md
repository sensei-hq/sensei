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
- [ ] 3. Screens: repo diagram route + branch picker (`is_default`
      default) + lens switcher on `CodeMap`; team scoping for the
      world view.
- [ ] 4. Tracked branches: authority-gated add/remove screen through
      `set_graph_branch`; the picker reflects the change.
- [ ] 5. Schema view: ER canvas over `graph_schema_*`.
- [ ] 6. Gates green, commit, push.
