# Checkpoint

**Slice:** v0.11.0 released; graph re-indexing on the new binary.

## State

**v0.11.0 is tagged, pushed and installed.** CLI, daemon and `/Applications/Sensei.app`
all report 0.11.0. The repository was REBUILT — see below.

| gate | result |
|---|---|
| senseid suite | **3,312 pass / 0 fail / 51 ignored** |
| bootstrap · app · dojo | 175 · 1,700 · 1,535 — all pass |
| clippy `--workspace --all-targets -D warnings`, fmt `--all` | clean |
| leak guard | 28/28 self-test, `--audit` clean |
| `check-ddl-comments` | clean, wired into `test-fast` |
| **Tauri e2e** | still the open gap (unchanged, not re-run) |

## The repository was replaced, not rewritten in place

`sensei-hq/sensei` is a NEW repository. The pre-scrub original is
`sensei-hq/sensei-archive` — **private and archived, and it must stay that way**;
it still holds the unscrubbed history and issues. Local mirror backup at
`~/sensei-history-backup/`.

- 4,048 commits rewritten; blob counts identical before and after, so nothing
  was dropped to achieve it. 9 commit messages cleaned.
- Issues #1–#200 migrated with NUMBERS PRESERVED (173 distinct `#NNN` references
  in commit messages would otherwise have broken). 91 open / 109 closed.
- The 5 git stashes were converted to `wip/stash-0..4` **local branches** before
  the rewrite and deliberately NOT pushed — filter-repo rewrites refs, not the
  stash reflog, so 4 of 5 would have been lost.
- **Releases were not migrated** (pre-release, judged not to matter), so
  `brew install` 404s until a release is cut. v0.11.0's CI is building now.

## What landed in 0.11.0

- **`architecture` metric family** — the first metrics that read the code graph:
  `graph_confidence`, `public_surface_ratio`, `symbol_size_p95`. Module-level
  metrics (coupling, cycles) are deliberately unbuilt: the module graph collapses
  to 13 module→module pairs (#156, #146/#151/#152).
- **The edge verdict is derived at INSERT**, not only on merge. It was NULL on
  every newly-created edge; the views recompute at query time, so the
  materialised column was verified by nothing.
- **434 lines of prose out of 39 column lists** → `comment on column` +
  `docs/database/<schema>.md`, with a gate.
- **Transcript fixture corpus** + anonymiser that verifies its own output.
- `copilot`/`vscode` added to `assistant_family` — both adapters returned a value
  the enum rejected.

## In flight right now

Full re-index after `TRUNCATE sensei.files, sensei.nodes, sensei.edges CASCADE`
(also cleared `inference.drift_items`). Truncating `files` is REQUIRED —
`pipeline.rs:121` skips any file whose stored mtime matches, so leaving it would
have produced an empty graph.

Before: 123,569 files · 1,182,930 nodes · 4,079,408 edges.
At last check: 95,498 files · 108,890 nodes · 396,355 edges, 15 active queries.

**89–91% of new edges carry a verdict**, which is the insert-path fix working
from scratch — so `scripts/backfill-edge-verdicts.sh` is now moot for this
machine. It stays for any database not re-indexed.

## Next command

    psql "postgresql://localhost:5432/sensei" -tAF'|' -c "select count(*) from sensei.nodes"

Watch it settle near 1.18M nodes / 4.08M edges, then confirm the architecture
metrics compute:

    psql "postgresql://localhost:5432/sensei" -tAF'|' -c "
      select m.key, pm.value, pm.props from sensei.project_metrics pm
        join sensei.metrics m on m.id = pm.metric_id where m.family='architecture'"

## Open

- **Tauri e2e** does not compile clean and is outside the static gates (#187).
- **v0.11.0 CI** — confirm the release workflow publishes assets and renders the
  tap; 0.10.0 tagged and published nothing.
- **`develop` is not merged to `main`** yet for this release.
- Two CLAUDE.md release-checklist items name things that do not exist in this
  tree: the `docs_match_code::the_website_copies_match_the_docs` gate, and the
  dual `docs/skills/` + `src/assets/skills/` copies (skills live in
  `.claude/skills/`). Either build them or correct the checklist.
