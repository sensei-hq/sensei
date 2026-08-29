# 支 · Bootstrap · Probing

**Segment:** 01 · Bootstrap
**Route:** initial splash window (before Observatory main)
**Source mockup:** [`lib/setup/bootstrap-splash.jsx`](../../mockups/Sensei/lib/setup/bootstrap-splash.jsx) → `Splash` (state = `probing`)
**App file:** `app/src/routes/bootstrap/+page.svelte` (or the Tauri splash surface)

## Purpose

Probing is what the user sees the first time (and every time
after a wipe) sensei starts up on their machine. Six gates run
sequentially — Homebrew, Postgres, Ollama, components, DB, daemon
— each checking that the foundation is present and healthy. The
user's job is to *watch and trust*, not to do anything. Every gate
states what it's for and what breaks if it's missing so the wait
teaches, not just spins.

The probing screen never shows a "getting started" nag. If
everything comes up green, it collapses to the "all green" state
([[screen/bootstrap-green]]) and continues into the Observatory
first-run flow.

Kanji is 支 — *support / foundation*.

## Data invariants

- `GET /api/bootstrap/status` returns:
  ```json
  {
    "state": "probing" | "green" | "remediating" | "failed",
    "gates": [
      { "id": "homebrew", "kanji": "一", "name": "Homebrew",
        "detail": "package manager",
        "status": "probing" | "pass" | "remedy" | "fail",
        "why": "…", "ifMissing": "…",
        "started_at": iso?, "resolved_at": iso? }, …
    ],
    "elapsed_ms": N,
    "download_size_estimate_mb": N?
  }
  ```
- Gate ids: `homebrew`, `postgres`, `ollama`, `components`, `db`,
  `daemon`. Order is fixed — the sequence tests the dependency
  chain top-down.
- **Ollama is a soft dependency.** The daemon ships with an
  embedded ollama, so a system ollama that's absent or
  unreachable falls to `remedy` **without blocking** subsequent
  gates. The user can continue into the app; narration-cache and
  other inference chains route to embedded gemma4. Upgrading a
  soft-remedy gate later is a "Re-check" away. Deferred
  refinement: better copy on the row that says "embedded ollama
  available; system ollama would give you access to more models."
- Gates run in parallel where the dependency graph allows; the
  UI shows them running as the daemon reports.
- Bootstrap runs on daemon start and via a "Re-check" action
  reachable from the Observatory (see the journey map §2.1
  resolved list).
- Instrumentation is captured via the `BootstrapTrace` /
  `TraceRecorder` primitives (see
  (memory: project_p2_sweep_2026_07) memory) so each gate's timing is
  observable after the fact.
- Offline / proxy path: if network is required and unavailable,
  Homebrew and Ollama gates fall to `remedy` state with a
  guided-manual-fallback note (see the journey map's approach —
  one hand-run step, then automatic).

## Signals shown

| Element | Value |
|---|---|
| Header title | `Sensei · setup` or similar; not marketing |
| Header note | "Every gate states what it's for and what breaks if it's missing." (or the mockup's phrasing) |
| Gate row × 6 | kanji numeral · name · detail · status indicator |
| Gate status: probing | animated spinner + `probing…` label |
| Gate status: pass | green check + `ok` |
| Gate status: remedy | amber icon + short remediation note + `manual step` chip |
| Gate status: fail | red icon + specific error + retry button |
| Why line (per gate) | short "this is why we need X" sentence |
| If missing (per gate) | short "if missing: Y breaks" sentence |
| Elapsed timer | mono, small, top-right |
| Download size estimate | mono, small, top-right when applicable |
| Re-check button | always present in Observatory, but hidden during probing |

## Done gate

- On a clean install, the six gates run and reach `pass` or
  `remedy` without the user having to click anything except a
  manual-fallback confirmation when a gate genuinely can't
  auto-resolve.
- Each gate's `why` and `ifMissing` render before the gate hits
  `pass` — the user learns while they wait.
- The elapsed timer reads real elapsed time from the daemon
  (`BootstrapTrace`), not a client-side clock.
- Bootstrap re-runs cleanly via the Observatory Re-check action
  — no residual UI state from a prior run.
- Offline path: if the machine is offline, Homebrew / Ollama
  gates fall to `remedy` with a "you'll need to run this
  yourself" note; user completes; sensei continues.
- After all gates reach `pass`, the screen transitions to
  [[screen/bootstrap-green]] without a delay.
- BootstrapTrace timings are queryable after the fact for
  diagnosis.

Optional check:
```
curl -s http://localhost:7744/api/bootstrap/status \
  | jq '{state, gates: (.gates | map({id, status}))}'
# expected during probing: gates with mixed statuses;
# expected on green: every gate.status == "pass"
```

## Wrong gate

- **The whole screen is a spinner with no per-gate breakdown.**
  Users can't tell why they're waiting.
- **Gate rows show `probing` forever with no timeout.** Each
  gate has a bounded wait; on timeout the row falls to `fail`
  with a clear reason.
- **Marketing copy in the header** ("Welcome to Sensei —
  building the future of…"). This surface teaches by watching;
  no marketing.
- **A gate hits `pass` but the summary count says `5 of 6`.**
  Aggregate query and gate query diverged.
- **Re-check triggered from the Observatory doesn't re-run every
  gate.** Re-check must be a full re-run, not "just the failed
  ones", because a downstream gate's health can shift a
  previously-passing upstream gate.
- **Post-green transition delayed by > 500ms.** Feels stuck.
- **`ifMissing` says "your setup will fail".** Too vague. It
  needs to name the specific downstream capability that breaks.

## Related

- [[screen/bootstrap-green]] — the collapsed all-green state
- [[screen/first-run-scan]] — where the flow goes next
- [[pipeline/capture]] — the daemon that these gates prep
- (memory: project_p2_sweep_2026_07) (memory) — BootstrapTrace foundation
