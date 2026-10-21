# Roadmap — Platform plane (P1–P3)

Distribution (P1), website (P2), harness/CI (P3). Status legend:
[`README.md`](README.md).

> Per-module statuses are filled in Phase 3 from the legacy coverage map and the
> plan gap analysis (both deleted on 2026-10-21; they live in git history).
> Structure is final.

| ID | Module | One-line "done" | Status |
|---|---|---|---|
| P1 | Distribution | One command installs sensei on macOS, Linux and Windows | 🚧 Phase 3 |
| P2 | Website | The site says only what is true, and sells the built product | 🚧 Phase 3 |
| P3 | Harness & CI | No change reaches main without tests, lint, fmt and e2e | 🚧 Phase 3 |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| D-11 | website presents unbuilt as shipped (one claim already dropped) | P2 |
| D-12 | dependency drift: `@rokkit/*`, `@kavach/*` held back by lockfiles | P3 |
| D-13 | 28 medium osv advisories (transitive; none code-fixable) | P3 |
| P-5 | Makefile/Homebrew are macOS-only; Linux service/data-dir/shell assumptions unresolved — design first, then distribute (decision 7) | P1 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

P1's three-target design is written before the implementation distributes across
macOS / Linux / Windows (decision 7, reorg §10.3). P2 is written after the
roadmap statuses so its claims trace to evidence. P3's e2e reds were triaged and
baselined in #134 (closed 2026-10-18); a green-main claim still rests on that baseline.