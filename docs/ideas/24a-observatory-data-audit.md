---
name: Observatory Data Audit
status: analysis
origin: docs/ideas/24-desktop-observatory.md
date: 2026-04-19
description: Trace every desktop insight back to required data — what we can capture, what's blocked, what needs ACP feature requests
---

# Observatory Data Audit

Every insight in the desktop observatory needs data. If we can't capture the data, the insight is decoration. This document maps each insight to its data requirement, current capture status, and the gap.

## Status Legend

- ✅ HAVE — data exists in daemon store today
- 🔧 BUILDABLE — we can capture this with changes to hooks/commands/daemon
- 🚫 BLOCKED — requires ACP capability that doesn't exist (needs feature request)

---

## 1. Home Page — "How am I doing?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| FTR score | session outcomes (completed/partial/blocked) | ✅ | sessions.outcome exists, FTR computed in row_to_session |
| Session count | session records | ✅ | sessions table |
| Turn count | turn events | ✅ | user-prompt hook logs "turn" events |
| Rework rate | revision_requested events | ✅ | user-prompt hook classifies corrections |
| Token usage per session | tokens_in, tokens_out | 🚫 | sessions table has columns BUT no ACP hook provides token counts |
| Cost per session | tokens × pricing | 🚫 | depends on token counts above |
| Quota remaining | Anthropic API quota check | 🚫 | no public API for quota introspection |
| Burn rate | cost over time | 🚫 | derived from cost per session |
| Tool adherence | tool_used events with is_mcp flag | ✅ | pre-tool hook could capture this (partially exists) |
| Active task/issue | workflow state | ✅ | workflow_state table |

**Actions available:**
- "Start session" → ✅ can trigger
- "View backlog" → ✅ can query issues
- "Index all" → ✅ can trigger

### Blocked: Token & Cost Tracking

This is the #1 blocker. Without token counts per session, we can't show cost, burn rate, or quota projections. Three possible approaches:

**Option A: ACP feature request** — Ask Claude Code for a `SessionEnd` or `PostSession` hook that includes `{tokens_in, tokens_out, cost}`. This is the clean solution.

**Option B: API key introspection** — Anthropic's API has usage endpoints for API keys. Claude Code users use OAuth, not API keys, so this may not apply. Worth investigating.

**Option C: Estimate from turn count** — Rough estimate: avg tokens per turn × turn count. Inaccurate but better than nothing. Could calibrate from known sessions.

---

## 2. Sessions — "What happened?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| Session list with outcomes | sessions table | ✅ | exists |
| Event timeline | events table ordered by time | ✅ | exists |
| Tool call name + params | pre-tool hook captures tool + params | 🔧 | hook exists but may not store params in events |
| Tool call response/output | post-tool hook with response content | 🚫 | PostToolUse hook gets exit code, not response content |
| Which mindsets applied | "mindset_applied" event | 🔧 | not captured today — add to command prompts |
| Which personas applied | "persona_applied" event | 🔧 | not captured today — add to command prompts |
| Rules adherence per session | "rule_checked" / "rule_violated" events | 🔧 | not captured — add to command/skill prompts |
| Session duration | started_at, completed_at | ✅ | exists |
| Correction context (what was wrong) | enriched revision_requested event | 🔧 | event exists but doesn't capture WHAT was corrected |

### Buildable: Mindset/Persona/Rule Tracking

We control the command prompts. We can add explicit instructions:

```
When applying a mindset, call:
  log_event(type="mindset_applied", data={"mindset": "analyst", "session_id": "..."})

When checking a rule, call:
  log_event(type="rule_checked", data={"rule": "TDD", "adhered": true})

When applying a persona's questions, call:
  log_event(type="persona_applied", data={"persona": "ai-driven-developer"})
```

**Prerequisite:** `log_event()` MCP tool must be available (issue #80). Currently the events API exists but the MCP tool wrapper isn't in the tool list.

### Blocked: Tool Response Capture

Claude Code's PostToolUse hook provides:
- Tool name
- Exit code (for Bash)
- Whether the tool was allowed/denied

It does NOT provide:
- The actual response content (what the tool returned)
- Token cost of the tool call

**Feature request needed:** PostToolUse hook should include a `response_preview` field (first N chars of the tool response) or a `response_hash` for dedup detection.

---

## 3. Projects — "What am I working on?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| Symbol counts | graph.count_symbols() | ✅ | exists |
| Graph visualization | graph.get_nodes(), get_edges() | ✅ | exists |
| Complexity hotspots | hierarchy_nodes.complexity | ✅ | exists |
| Dead code candidates | exported but 0 callers | 🔧 | callers_of() exists, need a "zero callers" query |
| Duplicates | pattern_detector duplicate detection | ✅ | exists |
| Doc drift | graph.find_drifted_docs() | ✅ | exists |
| "Tell Claude" action | generate prompt + copy to clipboard | 🔧 | UI-only, no daemon change needed |

**All buildable.** This is the strongest section — graph data is already rich.

---

## 4. Libraries — "What docs does Claude have?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| Indexed libraries list | lib_meta table | ✅ | exists |
| Section/component count | lib_docs table | ✅ | exists |
| Freshness (indexed_at) | lib_meta.indexed_at | ✅ | exists |
| Usage in sessions | tool_used events where tool=get_lib_docs | 🔧 | events exist but not filtered by library name |
| "Simulate tool call" | call MCP tool from UI, show response | 🔧 | daemon has /api/mcp/call endpoint |
| Stale detection | indexed_at vs threshold | ✅ | derived |
| Auto-detect from code | scan imports, match against indexed libs | 🔧 | adapters extract imports, lib_meta has names |

### Buildable: Library Usage Tracking

The pre-tool hook already captures `tool_used` events. We need to enrich the data to include the library name when `get_lib_docs` is called:

```json
{"tool": "get_lib_docs", "is_mcp": true, "params": {"name": "rokkit"}}
```

This is a hook change, not an ACP limitation.

---

## 5. Tools — "What can sensei do?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| MCP tool catalog | mcp_list_tools() | ✅ | exists |
| Tool descriptions + params | mcp_list_tools() response | ✅ | exists |
| Usage stats per tool | tool_used events grouped by tool | ✅ | exists |
| "Try it" simulation | /api/mcp/call endpoint | ✅ | exists |
| Tool response preview | response from simulation | ✅ | via /api/mcp/call |
| Error rates | tool errors in events | 🔧 | post-tool hook could log errors |
| Response quality | user correction after tool use | 🔧 | correlate revision_requested with preceding tool_used |

---

## 6. Profiles — "What's helping and what's not?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| Mindset application count | mindset_applied events | 🔧 | add to command prompts |
| Persona application count | persona_applied events | 🔧 | add to command prompts |
| FTR impact per lever | correlate lever events with session FTR | 🔧 | needs lever events (above) + correlation query |
| Token impact per lever | token count per session with lever data | 🚫 | needs token counts (blocked) |
| Persona discovery from corrections | classify correction patterns | 🔧 | revision_requested events + NLP classification |
| Rule adherence stats | rule_checked events | 🔧 | add to command prompts |
| "Suggest improvements" | analyze session patterns | 🔧 | query + heuristics |

### Partially Blocked

FTR impact can be computed without tokens. But "this mindset costs 12% more tokens" requires token tracking, which is blocked by ACP.

**Workaround:** Show FTR impact (quality) without cost impact. Add cost column later when token data becomes available.

---

## 7. Benchmarks — "Is sensei helping?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| Task configuration | benchmark task definitions | 🔧 | need to build task store |
| Run with/without sensei | automated session execution | 🚫 | no API to programmatically start Claude Code sessions |
| Comparison metrics | pre/post FTR, turns, time | 🔧 | if we can run sessions, metrics follow |
| Shareable report | export format | 🔧 | UI + markdown generation |

### Blocked: Automated Session Execution

Benchmarking requires running Claude Code sessions programmatically with controlled inputs. No ACP provides this today.

**Feature request needed:** A "headless" or "batch" mode for Claude Code that accepts a task description, runs a session, and returns metrics.

**Workaround:** Manual benchmarking — user runs tasks with and without sensei, app tracks the sessions, comparison is automatic.

---

## 8. Community — "How do I participate?"

| Insight | Required Data | Status | Gap |
|---------|--------------|--------|-----|
| Share benchmark results | export + upload | 🔧 | build export, hosting TBD |
| Share patterns/personas | export .sensei/ files | 🔧 | file export is trivial |
| Browse shared content | hosted catalog | 🚫 | needs a backend/registry service |
| Report issues | GitHub integration | ✅ | gh CLI exists |

---

## Summary: What to Build vs What to Request

### Build Now (daemon + hooks + commands)

| # | What | Where | Effort |
|---|------|-------|--------|
| 1 | `log_event()` MCP tool | routes.rs | small — wrapper around existing POST /api/events |
| 2 | Mindset/persona/rule tracking events | command prompts | small — add log_event() calls to commands |
| 3 | Enrich tool_used events with params | pre-tool hook | small — capture tool params in event data |
| 4 | Zero-callers query (dead code) | graph.rs | small — new query method |
| 5 | Library usage tracking | pre-tool hook enrichment | small |
| 6 | Correction context enrichment | user-prompt hook | medium — capture what preceded the correction |
| 7 | Profile impact correlation | store.rs | medium — query joining lever events with session FTR |
| 8 | Persona discovery heuristics | store.rs or command | medium — classify correction patterns |
| 9 | Token estimation fallback | store.rs | small — turns × avg tokens estimate |

### Feature Requests to ACP Providers

| # | What | Why | Target |
|---|------|-----|--------|
| FR-1 | **Token counts in session hooks** | Can't compute cost, burn rate, or quota without knowing tokens per session | Claude Code (Anthropic) |
| FR-2 | **PostToolUse response preview** | Can't show what a tool returned in session drill-down | Claude Code (Anthropic) |
| FR-3 | **Quota introspection API** | Can't show remaining quota or project burn-out date | Anthropic API |
| FR-4 | **Headless/batch session mode** | Can't run automated benchmarks without programmatic session execution | Claude Code (Anthropic) |
| FR-5 | **SessionEnd hook with metrics** | Clean way to capture session-level aggregates (tokens, cost, duration) at close | Claude Code (Anthropic) |

### Document for Future (not blocked, just deferred)

| # | What | Why deferred |
|---|------|-------------|
| D-1 | Community backend/registry | Needs hosted infrastructure, auth, moderation |
| D-2 | Shared benchmark catalog | Needs standardized task format + hosted comparison |
| D-3 | Cross-team analytics | Needs multi-user support, permissions |

---

## Recommended Build Order

1. **log_event() MCP tool** — unblocks all event-based tracking
2. **Enrich hooks** — tool params, correction context
3. **Profile tracking events** — mindset/persona/rule logging in commands
4. **Impact correlation queries** — connect events to outcomes
5. **Desktop Home page** — with available metrics (FTR, turns, rework — skip tokens until FR-1)
6. **Desktop Sessions page** — event timeline, profile adherence
7. **Desktop Profiles page** — impact view with available data
8. **Feature requests** — submit FR-1 through FR-5
9. **Token estimation fallback** — while waiting for FR-1
10. **Benchmarks** — manual mode first, automated if FR-4 lands
