---
name: Silent-Error Audit (daemon)
description: Codebase-wide audit of silently-discarded errors in crates/senseid, per the no-silent-errors rule. Classification + fix policy + hot-path exclusions.
date: 2026-06-18
---

# Silent-Error Audit — `crates/senseid`

Directed after the `node_kind` enum-drop bug (a real index failure hidden behind
`.ok()` for weeks). Rule: **never swallow an error silently — always log so it can
be inspected/fixed** ([[feedback_no_silent_errors]]).

## Method

A workflow classified every discarded-result site (`​.ok();`, `let _ = …`,
`Result`-masking `.unwrap_or_default()`/`.unwrap_or(…)`, `if let Ok(..)`/`Err(_)=>`
with no diagnostic) across 22 daemon files. Each site → **keep** (benign), **log**
(add `tracing::warn!`/`error!` with context, preserve behavior), or **propagate**.

**Result:** 22 files audited, **119 actionable**, 43 keep.

## Fix policy

- **Per-request / one-shot sites** (HTTP handlers, startup, install/uninstall,
  per-task/per-folder): add a `tracing::warn!(error = %e, …context…)` and keep the
  fallback (return None / empty / continue). Applied directly.
- **Per-edge loop sites** (`process.rs` import/call/parent/doc loops; `resolve.rs`
  covers loops; `community.rs` per-member update) — applied **per-call**, matching the
  established house style: `process.rs:639` already does a per-symbol
  `tracing::warn!` on `upsert_node` failure in this very loop. These fire only on a real
  `Err` (never in normal operation), so they're loud exactly when the DB is failing —
  which is when you want the signal — and consistent with the surrounding code.
- **Reclassified to keep** (the conservative classifier over-flagged): `languages/rust_lang.rs`
  `utf8_text(src).unwrap_or_default()` (×6). Tree-sitter already parsed the bytes as
  valid UTF-8, so `utf8_text` effectively cannot fail here; `""` is a fine degradation
  and adding a per-AST-node log to the parser for a can't-happen error is dead noise.
  Left as-is intentionally.

## Highest-value clusters

- **`api/handlers/mcp.rs` (24)** — nearly every MCP tool (`search`, `get_callers`,
  `get_project_summary`, `get_metrics`, …) does `.unwrap_or_default()` on its DB query,
  so a DB error returns an empty result indistinguishable from "nothing found". This is
  a prime "why are the tools silently empty?" source. Fixed.
- **`tasks/handlers/resolve.rs` (12)** & **`process.rs` (28)** — the index/edge-resolution
  path (the `#57`/`#60` area): DB query masks (`unwrap_or_default` → empty node/edge sets)
  and write swallows (`.ok()` on `upsert_node`/`insert_edge`/`set_*`). Same class as the
  original `node_kind` bug.
- **`installer/removal.rs` (8)** — uninstall reported success while discarding failed
  `remove_dir_all`/`remove_file`; now logs + records errors.

## Already fixed before this pass
`api/handlers/sessions.rs:173` `ingest_hook_event` — `let _ = insert_hook_event(…)`
swallowed capture-insert failures (commit `6638bc68`).

## Follow-up
- Reword the `clean_sensei_from_mcp_file` comment in `installer/install.rs` — "accumulates"
  overstates it (it's one stale legacy entry, not unbounded growth; see [[project_capture_watchdog]]).
