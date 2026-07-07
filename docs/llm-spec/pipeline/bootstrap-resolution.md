# 支 · Pipeline · Bootstrap resolution

**Owner files:**
- Trace / probe: `crates/bootstrap/src/health/trace.rs`,
  `crates/bootstrap/src/probe/*` (see
  [[project_p2_sweep_2026_07]] BootstrapTrace foundation)
- Resolvers per component: `crates/bootstrap/src/resolvers/{homebrew,postgres,ollama,components,db,daemon}.rs`
- Hardware inspector: `crates/bootstrap/src/hardware.rs`
- Tauri sidecar (app-only mode): `app/src-tauri/src/bootstrap_sidecar.rs`
- API: `crates/senseid/src/api/handlers/bootstrap.rs`

**Companion design doc:** [`docs/archive/ideas/26-bootstrap-and-dependencies.md`](../../archive/ideas/26-bootstrap-and-dependencies.md).

## Purpose

Bootstrap runs on every launch — not just first install. On a
healthy machine it's a fast-path (< 2s) probe cycle that
confirms the foundation is intact and continues into the
Observatory. On a degraded machine it's the guided-resolution
flow — install what's missing, start what's stopped, pull the
right model for the hardware, then continue.

The **five phases** (see [[screen/bootstrap-probing]] for the UI):

| # | Phase | Owns |
|---|---|---|
| 1 | Package manager | Homebrew (macOS), winget (Windows), apt (Linux) |
| 2 | Core components | sensei formula, dbd, related tools |
| 3 | Database | PostgreSQL, DDL sync |
| 4 | Local models | Ollama daemon + gemma4 + nomic-embed-text |
| 5 | Sensei daemon | senseid startup + first health signal |

Each phase: **detect → resolve → verify** cycle. Detection is
free (no side effects); resolve invokes the installer if
detection fails; verify confirms the resolve step actually took.
If any step falls to `remedy`, the user gets a specific
manual-step suggestion.

Kanji is 支 — *support / foundation*.

## Data invariants

### BootstrapTrace

Every bootstrap run records a `BootstrapTrace` — a structured
event stream of `{ phase, step, status, ts_ms, duration_ms,
detail }` rows. Retained in memory during the run; the last N
runs persisted in `sensei.bootstrap_runs` for diagnosis.

### Fast-path vs full-path

- **Fast-path**: every phase's `detect` step returns healthy.
  Sub-2s from launch to Observatory. This is the steady-state.
- **Full-path**: any phase's `detect` fails and needs `resolve`.
  Latency varies (Ollama model pull can take minutes). Progress
  is streamed to the UI.

### Hardware tiers

Ollama model recommendation reads system RAM + CPU:

| RAM | Recommendation | Backing model |
|---|---|---|
| ≥ 32 GB | `advanced` | gemma3:27b (or similar 20B+) |
| 16 GB | `balanced` | gemma3:12b |
| ≥ 8 GB | `lite` | gemma3:1b + nomic-embed-text |
| < 8 GB | `no-inference` | gateway routes remote-only OR skips optional inference |

User can override the recommendation in Preferences →
Inference.

### Tauri sidecar (app-only mode)

If the user opens the desktop app before the daemon is
installed, the Tauri sidecar handles the initial detection
steps that don't need the daemon:

- Homebrew detection + install prompt
- Formula check
- Hardware inspection
- Ollama detection + optional pull
- Service starts

The sidecar's job is only to reach the point where the daemon
can boot; once senseid is up, bootstrap continues from the
daemon side. This keeps the desktop-first install experience
working without a chicken/egg.

### Ollama as soft dependency

The daemon ships with an **embedded ollama** binary, so the
Ollama phase resolves to `pass` even when a system-wide Ollama
is absent. If the user later installs a system Ollama with
larger models, sensei can prefer that on next boot. See
[[screen/bootstrap-probing]] "Ollama is a soft dependency".

## Signals produced

| Signal | Consumer |
|---|---|
| Phase status stream | Bootstrap probing UI + Bootstrap green |
| BootstrapTrace | `sensei.bootstrap_runs` for diagnosis |
| Hardware profile | Preferences → Inference recommendation |
| Health baseline | The signal every downstream health-check reads (see [[project_capture_watchdog]]) |
| Model available list | Inference chain configuration |

## Done gate

- Fast-path total time ≤ 2s on a warm machine.
- Every phase produces a specific `remedy` message (not a generic
  "failed") when detection returns unhealthy.
- Ollama phase reaches `pass` on a fresh install without system
  Ollama, using embedded ollama.
- Hardware tier detection matches actual machine RAM.
- Tauri sidecar can complete phases 1–3 without a running
  daemon.
- Re-check triggered from the Observatory re-runs every phase.
- BootstrapTrace persists for at least the last 10 runs.

Optional check:
```
curl -s http://localhost:7744/api/bootstrap/status | jq '.gates | map({id, status})'

psql -A -t -c "select id, started_at, ended_at, status
                 from sensei.bootstrap_runs
                 order by started_at desc limit 5" -d sensei
```

## Wrong gate

- **Fast-path takes > 5s** on a warm machine. Detection step
  is doing side effects OR the health probe is doing too much.
- **Phase reaches `pass` before its resolve step actually
  completed.** Verify step wasn't run.
- **Hardware tier recommends `advanced` on an 8GB machine.**
  Detection wrong; user's model won't fit.
- **Sidecar-only mode falls into a state the daemon can't
  reconcile.** Sidecar and daemon must agree on the persisted
  state shape.
- **BootstrapTrace vanishes on daemon restart.** Should persist
  in `sensei.bootstrap_runs`.
- **Re-check doesn't re-run every phase.** Should be a full
  re-run.
- **Ollama phase falls to `fail` when embedded ollama is
  functional.** Detection didn't consider the embedded binary.

## Related

- [[screen/bootstrap-probing]] — UI for the phases
- [[screen/bootstrap-green]] — terminal state
- [[pipeline/inferencing]] — Ollama model recommendation feeds
  chain config
- [[project_capture_watchdog]] (memory) — health baseline
- [[project_p2_sweep_2026_07]] (memory) — #39 BootstrapTrace
  foundation shipped
- [[archive/ideas/26-bootstrap-and-dependencies]] — source design
