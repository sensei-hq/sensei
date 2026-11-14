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

- In: the health check (the package manager — Homebrew or winget — plus postgres, ollama,
  sensei, database, daemon); per-component
  resolvers with dependency-aware order and fallback remedies; hardware-tier detection and
  model provisioning for the tier; the probing / remediating / all-green splash states; the
  Re-check entry in the Observatory; Tauri sidecar bootstrap in app-only mode.
- Out: folder and workspace scanning (M3); the free-navigation preferences surface (M2); the
  first-run scan screen (M2); dojo auto-discover (M3).

## Surfaces

- **App splash** — the `(config)/setup` wizard stages (`welcome → scan → roots →
  assistants → done`, `app/src/routes/(config)/setup/`); the all-green (`完`) state is the
  last stage.
- **`GET /api/health`** — the daemon's readiness: the six gates resolved ready plus the
  hardware tier (`api/routes.rs:581`, `crates/bootstrap/src/health/resolvers/*.rs`). The
  `GET /api/bootstrap/status` state machine (`state` + one row per gate) was designed and
  **dropped** in favour of `/health`; it has no route.
- **Resolvers** — `crates/bootstrap/src/health/resolvers/{postgres_install,ollama_install,
  sensei_install,db_setup,daemon_start,brew_helpers,service_cascade}.rs`;
  one resolver per component; a resolver whose dependency failed is skipped in this pass.
- **Hardware inspector** — `crates/bootstrap/src/hardware.rs`; model tier feeds M5's
  Preferences → Inference recommendation.
- **Tauri sidecar** — `app/src-tauri/src/commands/bootstrap.rs` (sidecar spawn in
  `app/src-tauri/src/lib.rs`); app-only mode that bootstraps
  without a full daemon stack.

## Data invariants

- `components[*].status ∈ {pending, checking, installing, ready, failed}` and
  `packageManager.status` uses the same set; top-level `status ∈ {checking, resolving, ok,
  needs-action}`. The all-green state is reached **only** when all six are `ready` and the
  top-level `status` is `ok`; `needs-action` always carries a `remedy` (INV-1,
  `crates/bootstrap/src/health/types.rs:130-141`).
- A component's wait is bounded — on timeout it falls to `failed` with a reason, never `checking`
  forever.
- Resolvers form a dependency graph: a dependent resolver is skipped while its dependency is
  still failing, and runs within the same pass once the dependency recovers.
- Hardware tier ∈ {lite, balanced, advanced, no-inference}; the tier→model map is exhaustive —
  every tier provisions its embedded defaults; a tier with no installed models is reported as
  `no-inference`, not silently empty.
- `BootstrapTrace` records each gate's `started_at` / `resolved_at` and total elapsed; the
  splash timer reads real daemon time, not a client-side clock.
- Offline path: Homebrew / Ollama report `failed` and the payload carries a `remedy` with a
  "run this yourself" note; the run never reports all-green because it is offline.

## Signals shown

| Element | Value | Meaning | Decision it drives / next step |
|---|---|---|---|
| Gate rows (×6) | kanji · name · detail · status | what's being checked and why it matters (`why`, `ifMissing`) — the wait teaches | a red row names its fix: wait while sensei installs it, or run the `ifMissing` command it shows |
| Elapsed timer | mono, top-right | real elapsed from `BootstrapTrace` | if it runs long with no row moving, take the Continue link |
| Download size estimate | mono, top-right (when applicable) | how much a pending install will pull | none — candidate to cut (informational; no choice is offered) |
| Re-check | Observatory entry | re-runs bootstrap on demand; returns to Observatory on green, not first-run | after fixing a red gate by hand, re-run bootstrap |
| All-green hero | `完` large + "everything ready · opening your workspace" | machine ready; auto-transition ≤ ~800ms | none needed: sensei opens the folder scan on its own |

## Done gate

- A fresh install reaches all-green with **no manual toolchain work** (a single manual
  confirmation is allowed only when even Homebrew is absent).
- A healthy machine's probe cycle completes in <2s and continues — note the fast path vs full
  path distinction in [`bootstrap-resolution.md`](../../../../spec/pipeline/bootstrap-resolution.md).
- Re-check from the Observatory re-runs cleanly with no residual UI state from a prior run.
- An offline machine gets remedy notes, the user completes them, and sensei continues.

```bash
# green run — `status` is `ok` and every component (and `packageManager`) is `ready`; probe cycle fast
curl -s http://localhost:7744/api/health | jq '{status, components: (.components | map({id, status}))}'
# timed form: components[].id ∈ {postgres, ollama, sensei, database, daemon} plus
# packageManager.id (homebrew | winget) — Homebrew is never a components[] row (INV-2/INV-3)
```

## Wrong gate

- **All-green renders while a component is `failed` or the top-level status is `needs-action`** —
  the aggregate is wrong; some components were counted ready when they weren't.
- **Component rows hang in `checking` with no timeout** — the user can't tell why they're waiting.
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