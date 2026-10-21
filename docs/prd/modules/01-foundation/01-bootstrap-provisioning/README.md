# Bootstrap & Provisioning · M1

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)), team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

The user watches sensei's machine check itself — not clicks a wizard. On first launch
(and every launch after) sensei verifies the foundation is intact: package manager,
Postgres, a local model runtime, its own components, the database, and the daemon.
Anything missing gets installed or started automatically; anything that can't be
auto-resolved becomes one calm "run this yourself" note. When everything is green the
splash hands over to the folder scan on its own, within a beat. The whole job of this
module is *frictionless entry*: "install once and forget" (persona 1) and "start fast on
a repo the team has history on" (persona 2).

## In scope / out of scope

- In: six-gate health check (Homebrew, Postgres, Ollama, components, DB, daemon); per-component
  resolvers with dependency-aware order and fallback remedies; hardware-tier detection and
  model provisioning for the tier; the probing / remediating / all-green splash states; the
  Re-check entry in the Observatory; Tauri sidecar bootstrap in app-only mode.
- Out: folder and workspace scanning (M3); the free-navigation preferences surface (M2); the
  first-run scan screen (M2); dojo auto-discover (M3).

## Surfaces

- **App splash** — `app/src/routes/bootstrap/+page.svelte` renders probing, remediating, and
  all-green (`完`) states; a Continue link is the escape hatch if the auto-transition stalls.
- **`GET /api/bootstrap/status`** — the daemon's bootstrap state machine: top-level
  `state` (`probing|green|remediating|failed`) plus one row per gate
  (`id, kanji, name, detail, status, why, ifMissing, started_at, resolved_at`).
- **Resolvers** — `crates/bootstrap/src/resolvers/{homebrew,postgres,ollama,components,db,daemon}.rs`;
  one resolver per component; a resolver whose dependency failed is skipped in this pass.
- **Hardware inspector** — `crates/bootstrap/src/hardware.rs`; model tier feeds M5's
  Preferences → Inference recommendation.
- **Tauri sidecar** — `app/src-tauri/src/commands/bootstrap.rs` (sidecar spawn in
  `app/src-tauri/src/lib.rs`); app-only mode that bootstraps
  without a full daemon stack.

## Data invariants

- `status.gates[*].status ∈ {probing, pass, remedy, fail}`; the all-green state is reached
  **only** when every gate reports `pass`.
- A gate's wait is bounded — on timeout it falls to `fail` with a reason, never `probing` forever.
- Resolvers form a dependency graph: a dependent resolver is skipped while its dependency is
  still failing, and runs within the same pass once the dependency recovers.
- Hardware tier ∈ {lite, balanced, advanced, no-inference}; the tier→model map is exhaustive —
  every tier provisions its embedded defaults; a tier with no installed models is reported as
  `no-inference`, not silently empty.
- `BootstrapTrace` records each gate's `started_at` / `resolved_at` and total elapsed; the
  splash timer reads real daemon time, not a client-side clock.
- Offline path: Homebrew / Ollama gates fall to `remedy` with a "run this yourself" note; the
  machine never `fail`s the whole run because it is offline.

## Signals shown

| Element | Value | Meaning |
|---|---|---|
| Gate rows (×6) | kanji · name · detail · status | what's being checked and why it matters (`why`, `ifMissing`) — the wait teaches |
| Elapsed timer | mono, top-right | real elapsed from `BootstrapTrace` |
| Download size estimate | mono, top-right (when applicable) | how much a pending install will pull |
| Re-check | Observatory entry | re-runs bootstrap on demand; returns to Observatory on green, not first-run |
| All-green hero | `完` large + "everything ready · opening your workspace" | machine ready; auto-transition ≤ ~800ms |

## Done gate

- A fresh install reaches all-green with **no manual toolchain work** (a single manual
  confirmation is allowed only when even Homebrew is absent).
- A healthy machine's probe cycle completes in <2s and continues — note the fast path vs full
  path distinction in [`bootstrap-resolution.md`](../../../../spec/pipeline/bootstrap-resolution.md).
- Re-check from the Observatory re-runs cleanly with no residual UI state from a prior run.
- An offline machine gets remedy notes, the user completes them, and sensei continues.

```bash
# green run — every gate is pass; probe cycle fast
curl -s http://localhost:7744/api/bootstrap/status | jq '{state, gates: (.gates | map({id, status}))}'
# timed form: any gate.id in {homebrew, postgres, ollama, sdk, db, daemon} shows a status
```

## Wrong gate

- **All-green renders while a `remedy`/`fail` gate is pending** — gate aggregate wrong; some
  gates counted as pass when they weren't.
- **Gate rows hang in `probing` with no timeout** — the user can't tell why they're waiting.
- **Auto-transition never fires and Continue is absent** — the user is trapped on the splash.
- **Re-check from Observatory closes the app** — the fresh-install vs re-check context was
  handled wrong.
- **`完` renders as a fallback box** — the kanji font didn't load.
- **A resolver runs on a broken dependency** — e.g. `db_setup` attempts before Postgres is up.

## Scenarios

- [`scenarios/bootstrap.feature`](scenarios/bootstrap.feature) — health gate, resolution,
  hardware-tier provisioning, and the green handover.

## Open gaps

- No M1 entries are registered in [06-gaps.md](../../../../roadmap/06-gaps.md) today. Watch:
  non-embedded model pulls for a detected tier are a wiring surface (see bootstrap-resolution
  §Hardware tiers / §Ollama), and app-only sidecar parity with the daemon path is untested.

## Related

- [spec/pipeline/bootstrap-resolution](../../../../spec/pipeline/bootstrap-resolution.md) —
  the five phases, traces, fast vs full path, hardware tiers (legacy depth, folds here).
- [spec/screen/bootstrap-probing](../../../../spec/screen/bootstrap-probing.md) ·
  [spec/screen/bootstrap-green](../../../../spec/screen/bootstrap-green.md) — splash state UIs.
- [features/01-setup.md](../../../../features/01-setup.md) — the three-step entry gate story
  (stamped superseded; this spec is the replacement).
- [prd/02-objectives.md §01 Bootstrap](../../../02-objectives.md) ·
  [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/bootstrap/`; API: `crates/senseid/src/api/handlers/upgrades.rs`
  (`/api/upgrades`) and `crates/senseid/src/api/handlers/health.rs` (`/api/health`).