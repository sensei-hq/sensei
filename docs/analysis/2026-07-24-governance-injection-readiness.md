---
title: P3 governance injection (D-INJECT) — implementation readiness
date: 2026-07-24
status: CORE SHIPPED (develop 6ede1bdc) — SessionStart + PreCompact tier-aware push + endpoint + escape fix; relay-driver injection + authoring UI remain
relates: docs/design/instruction-delivery-model.md · docs/decisions.md (D-INJECT) · marketplace/plugins/sensei/hooks/
---

# D-INJECT — what's there now, the gap, the shape

**Decision (D-INJECT):** governance rules are *pushed*, not pulled — resolve the repo's
enforcement-tiered ruleset → inject at **SessionStart**, re-inject the **mandatory tier at
PreCompact**, and inject into the **relay driver** for daemon runs. Mandatory/required
always; advisory on-demand.

## Current state (verified in the marketplace hooks)

- **`hooks/session-start`** injects: `~/.sensei/rules.md` ("Global Rules (always-on)" —
  **user+general only**, daemon-materialized) + `${PROJECT_ROOT}/.sensei/rules.md`
  ("Project Guardrails", static file) + mindsets + personas + `state.yaml`. It literally
  says *"Repository-specific rules resolve live — call the `get_rules` MCP tool"* → the
  org/project/tech tiers are **pull-only**, so they reach the agent only if it remembers to
  call `get_rules`. That's the "not sticky" root cause.
- **`hooks/pre-compact`** injects `state.yaml` + `head -60 .sensei/rules.md` + mindsets +
  tools. `head -60` can **truncate the mandatory tier** and it never pulls the resolved
  repo mandatory rules → mandatory can silently drop post-compaction.
- **Daemon** already resolves rules (the `get_rules` MCP → some `/api/rules…` endpoint) and
  materializes `~/.sensei/rules.md` (user+general). The tiered repo resolution exists on the
  pull path; it is not pushed into the hooks.

## The gap → the shape (implementation-ready)

1. **Daemon: a rules-by-tier read the hooks can call with repo context.** Confirm/extend the
   `get_rules` endpoint to return the resolved repo ruleset grouped by enforcement tier
   (`mandatory | required | recommended | advisory`) for a given repo/cwd. (Check what the
   MCP `get_rules` maps to before adding — likely `/api/rules/{repo}` or similar.)
2. **SessionStart: push the tiered ruleset.** Have the hook fetch resolved rules for the
   project (curl the daemon with the repo id / cwd) and inject **mandatory + required**
   inline (always), listing recommended/advisory as available-on-demand. Keep the static
   `.sensei/rules.md` fallback for the daemon-down path.
3. **PreCompact: re-inject the mandatory tier reliably** — fetch just the mandatory rules
   (no `head -60` truncation) so they survive compaction.
4. **Relay driver:** when the daemon drives a run (drive is OFF today), the driver prompt
   must carry the same mandatory/required rules — the driver has no SessionStart hook.

## Shipped (2026-07-24, `6ede1bdc`)

Steps 1–3 done + live-verified: `governance::render_rules_tiers` + `GET
/api/knowledge/rules?format=md&tiers=…` (Markdown, no client jq) + the two hooks
push mandatory+required (SessionStart) / mandatory (PreCompact). En route, fixed a
**pre-existing bug**: `_lib.sh escape_for_json`'s hand-rolled bash escaping silently
failed on the large context in macOS bash → *invalid-JSON hook output*; now escapes
via `python3 json.dumps` (fallback hand-rolled). Both hooks emit valid JSON.

**Remaining (step 4 + authoring):** relay-driver rule injection (drive OFF today);
a rule-authoring surface. **Deploy gate:** `marketplace/` is a subtree — the hook
changes reach real sessions only after a marketplace sync + plugin update (the
installed `0.2.29` plugin is stale until then).

## Why the rest is staged

Correct governance injection is the "make rules stick" feature — getting the tier
resolution, the materialization format, and the three injection points right is a design
pass, not a mechanical edit, and it spans the marketplace hooks (bash subtree) + the daemon
rules API. It deserves fresh context. Everything above is the ready-to-build spec; the
run-supervision scope it follows is complete + live-verified (`docs/analysis/2026-07-24-
relay-stall-signal.md`, `…-relay-p1-livedrive` memory).
