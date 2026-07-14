# ✅ RESOLVED — Observatory · Instruments · Health (Slot 2)

**Status:** ✅ BUILT + LIVE-VALIDATED 2026-07-13 (the 2026-07-07 park below is STALE).
**Jerry decision (2026-07-13):** "A + add the 14d window" — but on investigation the daemon
+ frontend were ALREADY built (files dated 2026-07-08, after the park) and the endpoint
already serves the 14d-windowed rollup. So no build was needed; only a stale note.

## What's actually true now (live-validated 2026-07-13, debug daemon on real data)
- `sensei.assistant_tools` (unified tool registry) EXISTS + is populated: builtin 23,
  plugin_playwright_playwright 24, plugin_sensei_sensei 35, plugin_semgrep_semgrep 7,
  plugin_svelte_svelte 4.
- `GET /api/instruments/tools-health` (handler `tools_health::grid` → `pg_store::get_tools_health`)
  returns the correct `{sources:[...]}` rollup with a REAL 14-day window (from
  `activity.assistant_events` PostToolUse, `ts` epoch-millis cutoff — NOT the all-time
  `tool_usage_stats` view) + `share_invoked = tools_invoked_14d / tools_registered`.
  Live: builtin 23/21/17586 (share 0.91), sensei 35/13/29 (0.37), playwright 24/10/121 (0.42),
  semgrep 7/0/0 (connected/unused), and the un-probed degrade case works (a source with null
  registered → connected:false, share:null, no bar).
- Frontend built: `instruments/+page.svelte` Health tab (kanji 健) + `ToolHealthCard.svelte`
  (+ harness/spec) + `lib/state/mcp.svelte.ts` (`toolsHealth`, KPIs) + `api.ts` getter.

## Remaining (minor, NOT blockers)
1. **Duplicate svelte card** (spec "Duplicate source cards" wrong-gate): the svelte MCP is
   invoked under TWO tool-name namespaces — `mcp__plugin_svelte_svelte__*` (registered in
   assistant_tools → covered card, 4 registered) AND `mcp__svelte__*` (a second/older config,
   NOT in assistant_tools → falls into the `uncovered` CTE → phantom `svelte` card, null
   registered, 32 calls_14d). Root cause is capture/discovery registering the same server under
   two keys; the rollup faithfully reflects that. Fix belongs in tool_discovery (canonical
   server key / alias fold), not the rollup SQL. [[project_manifest_adapter_direction]] tool-discovery-trait.
2. Open filed bugs on this screen: #97 (L2 MCP defaults empty on first click), #98 (Health L2
   signal noise — most cards "dormant tool"), #96 (background-task visibility).

---
## (STALE — kept for history) PARKED — 2026-07-07 · AWAITS: Jerry (data-model decision)
**Gate that stopped it:** spec-doc-reviewer → `not-ready` (4 FAILs), then a data-model
investigation showed the screen's core signal has no truthful source. Not fixable by spec
edits alone.

## Why parked (root cause, not a wording nit)
The spec's L1 "MCP servers" grid centres on
`share_invoked = tools_invoked_14d / tools_registered` per MCP. Reality has **two disjoint
worlds that don't join**:

1. **Registry** `sensei.mcp_servers` — only 2 rows, both **Zed-discovered**
   (`mcp-server-context7`, `postgres-context-server`), `connection_state='unknown'`, no
   display metadata (name/kanji/publisher/note), no tool-usage captured. Registered-tool
   counts live in `sensei.mcp_tool_manifests` but are populated only by probing THESE Zed
   servers (`mcp_probe.rs`).
2. **Usage** `sensei.tool_usage_stats` — no MCP key; tools carry a Claude Code prefix
   (`mcp__plugin_playwright_playwright__…`, `mcp__svelte__…`, `mcp__…_sensei…`). These
   **Claude Code plugin MCPs are NOT in `mcp_servers`**, so their usage can't be attributed
   to a registered server, and they have **no `tools_registered`** anywhere (not probed).

Net: the registered servers have ~0 usage; the used servers aren't registered and have no
registered-tool count. `share_invoked` is uncomputable for the MCPs that matter. No descope
preserves the spec's central signal:
- L1 from `mcp_servers`+manifests → 2 Zed cards, ~0% share (useless/misleading).
- L1 from `tool_usage_stats` prefix-grouping → real cards but `tools_registered` unknown, so
  no share bar (the done-gate's core check) and no 14d window (call_count is all-time).

## Spec FAILs (from spec-doc-reviewer, secondary to the above)
1. L1 endpoint path drift: spec says `/api/observatory/mcp-servers`; real route is
   `/api/instruments/mcp-servers` returning `{servers:[...]}` with raw config columns, none
   of the spec's display/rollup fields.
2. L2 `tool-signals?mcp={id}` scoping doesn't exist (handler is param-free).
3. L2 `tool-insights?mcp={id}` scoping doesn't exist (handler is param-free).
4. Done-gate curls not executable (wrong paths/params).
Plus recs: H1 kanji should be 健 not 具; `connection_state`→`connected` normalization;
KPI strip missing from Signals-shown; add a worked-example row.

## AWAITS Jerry — the decision to unblock
Pick the MCP model this screen should reflect:
- **(A) Unify** — extend capture/discovery so Claude Code MCP servers (playwright, svelte,
  sensei, semgrep…) land in `sensei.mcp_servers` with probed `mcp_tool_manifests`, so
  `tool_usage_stats` joins by server and `share_invoked` is real. (Capture-pipeline work.)
- **(B) Redefine L1** — group `tool_usage_stats` by `mcp__<server>__` prefix, drop
  `share_invoked` (or redefine tools_registered as "distinct tools ever seen"), add a 14d
  window to usage capture. Cheaper; changes the spec's signal.
- **(C) Descope** — ship only the L2 per-tool health (already backed by
  tool-signals/tool-insights) and cut the L1 grid from this screen.

The L2 per-tool layer is buildable today (endpoints exist); only the L1 grid is blocked.

## What is already true (for whoever resumes)
- Routes: `/api/instruments/mcp-servers` (list), `/{id}/tools` (manifest), `/{id}/enabled`,
  `/refresh` — `crates/senseid/src/api/handlers/mcp_servers.rs`.
- `/api/observatory/tool-signals` + `/tool-insights` (param-free, all-MCP) —
  `observatory.rs:618/636`; derive logic in `tool_signals.rs`.
- Mockup: `docs/mockups/Sensei/lib/instruments-simple.jsx` → `instruments.jsx` `InstrumentsHealth`
  (H1 kanji 健). Data fixture `mcp-signals-data.js` (`note` field).
