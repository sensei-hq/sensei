# Gap register

The consolidated, IDed list of every known gap across the product — one line,
its evidence, its priority, and the module it blocks. This replaces the old
`plan/README.md` G1–G10 numbering and the `production-readiness.md` A–H
(now `07-quality.md`), which are absorbed here.

> **Status:** the register's full population is **Phase 3** of the reorg
> ([`../reorg.md`](../reorg.md) §12, §13). The ID scheme is final; the rows
> below are the ones already decided (2026-10-13) and the ones the plane files
> rely on. The complete F-/P-/T-/D-/M-/C-/X- register is folded in from reorg §12.

## ID scheme

| Prefix | Family |
|---|---|
| `F-*` | faults / silently-wrong behaviour |
| `P-*` | performance / scale |
| `T-*` | tests / gates / e2e |
| `D-*` | docs / claims / website truth |
| `M-*` | metrics / health / gating |
| `C-*` | capture / conventions / adapters / personas |
| `G-*` | graph / visualisation |
| `X-*` | cross-cutting / docs-repo |

Every row: **ID · module · one line · evidence · priority.** A gap with no
evidence is a suspicion, and gets labelled so.

## Decided gaps (2026-10-13)

| ID | Module | One line | Evidence | Priority |
|---|---|---|---|---|
| A-1 | M2 · M17 | app is kanji-only; the kanji ⇄ solar toggle the mockups + rokkit support is not wired | decision 10; reorg §12.10 | P3 |
| A-2 | M2 | the personal-preference surface is narrower than the config the app already supports | decision 10 | P3 |
| G-1 | M17 | graph/layered view unreadable at granular scale — no L0/L1/L2 level of detail | reorg §9.7 | P1 |
| G-2 | M4 · M2 · M3 | indexer has no ignore mechanism (no `.senseiignore` reader, no ignore config); it walks committed mockups / vendored / generated code and skews the graph + metrics. Ignore is stored repo config synced via the Dōjō, **seeded from the repo's own ignore sources** (`.gitignore`, qlty `.qlty/qlty.toml` `exclude_patterns`) — not a dotfile | decision 2026-10-13 | P2 |
| M-7 | M8 | no master-detail metric model, no `gating` field, gating eligibility undefined | reorg §9.6 | P1 |
| M-8 | M8 | coverage-gated presentation absent; thin metrics read as strong (#155 generalised) | reorg §9.6 | P1 |
| M-9 | M8 | no per-metric design/coverage record tying a metric to its chart and LOD | reorg §9.6 | P1 |
| C-11 | M3 | transcript-scanning review not scoped as a task | decision 10 | P1 |
| C-12 | M5 · M14 | inference-pipeline review not scoped as a task | decision 10 | P1 |
| C-13 | M3 | Claude "mods" (supersedes plugin + hooks) — install path + migration not verified | decision 10 | P1 |
| C-14 | M3 | multi-assistant expansion: Claude + OpenCode priority; others later; seam unspecified | decision 10 | P1 |
| C-15 | prd | personas (incl. manager + client/stakeholder viewpoints) — ✅ authored 2026-10-13 | `../prd/03-personas.md` | done |
| T-1 | T3 | tracker adapters (Jira/ADO) mockup-only; 2/7 gates buildable now, 2 scoped-read, 3 on #156 | reorg §9.2 | P1 |
| T-2 | T2 | join/membership flow dead ("Join" → 403) | plan gap analysis | P1 |
| F-3 | T1 | Dōjō sync never worked | production-readiness → `07-quality.md` | P1 |
| D-11 | P2 | website presents unbuilt as shipped (one claim already dropped) | site↔product audit | P2 |
| D-12 | P0(deps) | dependency drift: `@rokkit/*`, `@kavach/*` held back by lockfiles | production-readiness | P3 |
| D-13 | P0(deps) | 28 medium osv advisories (transitive; none code-fixable) | production-readiness | P3 |
| P-5 | P1 | Makefile/Homebrew macOS-only; Linux/Windows service + data-dir design first | reorg §10.3 | P1 |

## Remaining population (Phase 3)

The full register from [`../reorg.md`](../reorg.md) §12 spans the F- (faults),
P- (perf: e2e 23/130 red, `public.logs` 25 GB of a 37 GB DB, silent 1000% zoom
graphs), D- (doc conflicts and claims), T- (test coverage: no Rust tests in CI,
e2e flame-outs), M- (#153/#154/#155) and X- (backlog table) families. Each row
will name the module it blocks, so a plane file and this register always agree.

## How a gap closes

1. A `.feature` scenario in the module spec pins the behaviour (reorg §6).
2. The status flips in the plane file when there is evidence.
3. The row is removed from here and the work is tracked as a GitHub issue
   ([`../backlog.md`](../backlog.md)).