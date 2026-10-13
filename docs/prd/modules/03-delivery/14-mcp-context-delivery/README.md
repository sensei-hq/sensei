# MCP & Context Delivery · M14

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

MCP is how sensei becomes callable *inside* the assistant session. Every sensei
capability — search, graph, memory, governance, inference — is a declared MCP tool
with a **non-empty `defaults` contract**, so a caller never has to know an argument
name or a project id to get a useful answer. Context delivery is the other half:
instead of the assistant grepping or loading whole files, sensei assembles one
token-budgeted `context_pack` — diff-first over the code graph, traceability-boosted,
semantically backed-off, deduplicated against what this session already saw, and
levelled (L0 signature → L3 full source). The user feels this as *the assistant gets
the right context on the first turn* — and when it can't, it says so and falls back
to grep rather than inventing an empty answer.

> **Status honesty.** The tool catalog, the defaults contract, MCP dispatch and
> third-party discovery are real and well tested. The hybrid-search doc claims
> (`spec/pipeline/semantic-search.md` "roadmap (partial)") outrun the code: the
> lexical arm is a bare `ILIKE '%term%'` (**no FTS**), embedding coverage was 26% at
> 2026-08-05, and the doc-declared tables `sensei.node_summaries`,
> `sensei.session_context_log`, `sensei.node_embeddings` and `sensei.node_fts` are
> **not in the DDL** — so the L0–L3 resolution levels and the per-session dedup log
> are designed, not built. Doc-declared owner files
> (`crates/senseid/src/context/{pack,rank,resolution,dedup}.rs`,
> `crates/mcp/src/tools/*`, `crates/mcp/src/server.rs`, `crates/senseid/src/search/*`)
> **do not exist**; the real code lives in `crates/mcp/src/lib.rs` (tools declared
> inline) and `crates/senseid/src/api/handlers/query.rs`.

## In scope / out of scope

- In: sensei's own ~40 MCP tools, each declaring `name`, `description`,
  `input_schema`, `defaults`, `category`, `handler`; the argument-defaults contract
  (the daemon rewrites path/query/enum defaults server-side, the UI never invents
  them); third-party MCP discovery from `.mcp.json` / `.claude/settings.json` into
  `sensei.mcp_servers`; `context_pack` assembly over `sensei.edges`; hybrid search
  (lexical + vector + structural) with a confidence chip and a grep fallback; the
  per-session sent-log dedup; and per-call attribution into `activity.assistant_events`.
- Out: writing L2 logic-flow summaries synchronously in the request path (background,
  with a bounded timeout that falls back to L1); pretending to know code with no
  edges or embeddings (must emit a coverage-gap hint, never a fake empty); rebuilding
  raw capture (M3); enforcing behaviour for third-party MCP tools beyond
  discovery/tracking.

## Surfaces

- **API** — `GET /api/mcp/tools` (`mcp::mcp_list_tools`) · `POST /api/mcp/call`
  (`mcp::mcp_call_tool`) · `/api/instruments/mcp-servers`, `/{id}/enabled`,
  `/{id}/tools`, `/refresh`.
- **Daemon** — `api/handlers/query.rs::context_pack`; `api/handlers/mcp.rs::call`;
  `api/handlers/mcp_manifests.rs` (Playground shapes);
  `tasks/mcp_discovery.rs` (boot + every 60s, 24h stale prune); `tool_discovery.rs`.
- **MCP** — tools declared **inline** in `crates/mcp/src/lib.rs`: `context_pack`,
  `get_layered_context`, `search`, `get_patterns`, `get_project_summary`,
  `match_pattern`, `get_pattern_for`, `update_session`, `find_projects`,
  `get_user_for_project`, `set_stance`, `use_project`, `pause_run`, `start_run`,
  `record_outcome`, … grouped by category: Codebase · Session · Memory · Library ·
  Pattern · Governance · Inference · Gateway.
- **Data** — `sensei.mcp_servers`, `sensei.mcp_tool_manifests`, `sensei.assistant_tools`,
  `activity.assistant_events`, `sensei.nodes` (`embedding vector(384)`, HNSW
  `nodes_embedding_hnsw`), `sensei.edges`.

## Data invariants

- **Lead.** Defaults are the contract. Every tool's `defaults` is non-empty and
  appropriate; the Playground populates from it and never computes its own (the
  shipped `repoId`-empty bug came from violating this).
- **Lead.** Every tool call is attributed. Assistant calls carry `session_id`;
  Playground calls carry `user_id` + `playground=true`; both land in
  `activity.assistant_events` so signals can separate assistant patterns from user
  exploration.
- **Lead.** Budget is hard. A pack fits `max_tokens` (±100); omission is recorded in
  `omitted[]`, never silently truncated.
- **Lead.** Dedup respects level. A node already sent at the same-or-higher level is
  skipped; an upgrade (L1→L3) is still sent.
- **Lead.** Sensei is never worse than grep. On `overall_confidence == none` the
  response carries a `fallback_hint` and the caller / pre-tool hook reruns grep.
- **Lead.** Search and structural results reflect real graph coverage. Thin edges
  surface a coverage-gap hint rather than an empty "no callers".

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Tool declaration | Playground MCP tree / detail pane | `name: "sensei.search"` |
| `defaults` object | argument contract for the Playground form | `{ "query": "sensei.projects", "limit": 10 }` |
| Category group | Playground rail grouping | `Codebase` (8 tools) |
| Third-party MCP row | Health L1 grid | `postgres` from `.mcp.json` |
| `context_pack` result | pack handed to the assistant | `ContextPack { nodes, total_tokens, summary, omitted }` |
| `match_mode` | which arm matched | `fts` · `semantic` · `structural` · `hybrid` |
| `overall_confidence` | collapsed score | `high` = one hit ≥ 0.8 combined |
| Token-efficiency delta | Insights chip | "token spend down 30% since ranking landed" |

## Done gate

- Every tool in the tools surface declares a non-empty `defaults` matching its
  schema; `GET /api/mcp/tools?mcp=sensei` reflects the same categories and defaults.
- Third-party MCPs configured in Claude Code / Zed appear in `sensei.mcp_servers`
  within 60s of daemon start.
- Playground executes any tool round-trip without the user typing an argument; each
  call is logged with attribution.
- `context_pack` returns within `max_tokens` (±100) and skips content the assistant
  already saw this session.
- A conceptually-phrased `search` returns the right file without a substring match;
  the grep fallback fires on `none`.

```bash
curl -s "http://localhost:7744/api/mcp/tools?mcp=sensei" | jq '[.[] | select(.defaults|length==0)] | length'   # 0
```

## Wrong gate

- **A tool's `defaults` is `{}`** (the exact shipped bug) — every Playground query
  returns empty.
- **A Playground call succeeds but doesn't log to `assistant_events`** — user
  exploration corrupts assistant-behaviour signals.
- **A pack returns 20k tokens on an 8k budget** — budget enforcement broken.
- **Dedup lets the same node appear twice at the same level** — session-log query
  wrong.
- **Categorisation collapse** — all tools land under `Codebase` because the enum
  default is `Codebase`.
- **`get_callers` returns empty and grep isn't triggered** — a masked graph gap.
- **`overall_confidence == high` on irrelevant hits** — the semantic threshold is
  hallucinating similarity.

## Scenarios

- [`scenarios/mcp-surface.feature`](scenarios/mcp-surface.feature) — the tool
  catalog, the defaults contract, dispatch, resolution and third-party discovery
  (pinned).
- [`scenarios/context-delivery.feature`](scenarios/context-delivery.feature) — pack
  assembly, scopes and `context_pack` routes (pinned + `NO TEST` for the unbuilt
  dedup and levels).
- [`scenarios/search.feature`](scenarios/search.feature) — hybrid search, ranking,
  fusion and the grep fallback (pinned).

`NO TEST` scenarios are designed behaviour with no test yet; they feed the Phase 3
gap register (reorg §6.4).

## Open gaps

- [C-12](../../../../roadmap/06-gaps.md) — inference-pipeline review not scoped as a
  task (M5 · M14, P1).
- **MCP `search` gap** — `search` is symbol-only (lexical `ILIKE`, no FTS);
  `node_fts` / `hybrid.rs` absent. Listed in
  [`03-delivery.md`](../../../../roadmap/03-delivery.md) as **"D-8"** — that ID
  **collides** with [`reorg.md`](../../../../reorg.md) §12.9 `D-8` (Linux, P1); a
  re-ID is pending in the Phase 3 register.
- **[G4](../../../../reorg.md)** — re-IDed to M14 (§12.6); residual, no defect text.
- **[F-4 / F-5](../../../../reorg.md)** (M4) — graph 40.3% resolved (no target) and an
  unbounded parse stack; both directly undercut structural/hybrid search quality.
- **L0–L3 resolution levels + session dedup** — designed in
  `spec/pipeline/context-delivery.md`; the tables they need (`node_summaries`,
  `session_context_log`) are not in the DDL. No register row yet.

## Related

- [spec/pipeline/mcp-surface](../../../../spec/pipeline/mcp-surface.md) —
  the tool catalog + defaults contract.
- [spec/pipeline/context-delivery](../../../../spec/pipeline/context-delivery.md) ·
  [spec/pipeline/semantic-search](../../../../spec/pipeline/semantic-search.md) —
  pack assembly, hybrid search (both carry the roadmap/partial banner).
- [spec/pipeline/collective-intelligence](../../../../spec/pipeline/collective-intelligence.md) —
  cross-links M14/M15.
- [architecture/mcp](../../../../architecture/mcp.md) · [design/assistants](../../../../design/assistants.md).
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md](../../../01-vision.md#the-core-loop).
- Crates: `crates/mcp/src/lib.rs` · `crates/senseid/src/{api/handlers/query.rs,
  api/handlers/mcp.rs, tasks/mcp_discovery.rs, tool_discovery.rs}`.
