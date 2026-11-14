# MCP & Context Delivery · M14

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

MCP is how sensei becomes callable *inside* the assistant session. Every sensei
capability — search, graph, memory, governance, inference — is a declared MCP tool
with a **non-empty `defaults` contract**, so a caller never has to know an argument
name or a workspace id to get a useful answer. Context delivery is the other half:
instead of the assistant grepping or loading whole files, sensei assembles one
token-budgeted `context_pack` — diff-first over the code graph, traceability-boosted,
semantically backed-off, deduplicated against what this session already saw, and
levelled (L0 signature → L3 full source). The user feels this as *the assistant gets
the right context on the first turn* — and when it can't, it says so and falls back
to grep rather than inventing an empty answer.

MCP is also how **work arrives**. A tracker ticket is the unit of work handed to the assistant (D63):
the user gives the assistant a ticket key, the assistant fetches the ticket through a tracker-agnostic
tool the daemon serves, and that fetch marks the start of work and links the session. The same call
delivers the ticket's rules and the **targets** resolved for it (D62), and after a change the assistant
can ask which targets the change missed and what to fix (D61). The targets themselves — the ladder, who
sets them, what a blocking miss means — are owned by
[T3](../../04-team/03-dojo-workspace-planning/README.md).

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

- In: sensei's own MCP tools (56 declared in `crates/mcp/src/lib.rs` on 2026-11-03), each declaring `name`, `description`,
  `input_schema`, `defaults`, `category`, `handler`; the argument-defaults contract
  (the daemon rewrites path/query/enum defaults server-side, the UI never invents
  them); third-party MCP discovery from `.mcp.json` / `.claude/settings.json` into
  `sensei.mcp_servers`; `context_pack` assembly over `sensei.edges`; hybrid search
  (lexical + vector + structural) with a confidence chip and a grep fallback; the
  per-session sent-log dedup; per-call attribution into `activity.assistant_events`; and the three
  **work-delivery tools** below — ticket fetch with start of work, targets and ticket rules at start of
  work, and the missed-target handoff (D61–D63; not built).
- Out: writing L2 logic-flow summaries synchronously in the request path (background,
  with a bounded timeout that falls back to L1); pretending to know code with no
  edges or embeddings (must emit a coverage-gap hint, never a fake empty); rebuilding
  raw capture (M3); enforcing behaviour for third-party MCP tools beyond
  discovery/tracking; target configuration, evaluation and deviation acceptance (T3); tracker
  adapters and sync (T3); writing to a tracker (writeback is T3's, opt-in per workspace, D50).

## Surfaces

- **API** — `POST /api/mcp/call` (`mcp::mcp_call_tool`, the sensei MCP's
  dispatch) · `/api/instruments/mcp-servers`, `/{id}/enabled`, `/{id}/tools`
  (the server's own `tools/list`), `/{id}/call` (a tool run through the server
  over stdio `tools/call`), `/refresh`.
- **Daemon** — `api/handlers/query.rs::context_pack`; `api/handlers/mcp.rs::call`;
  `api/handlers/mcp_manifests.rs` (kind/summary for workspace tool stats);
  `tasks/mcp_probe.rs` (stdio session: probe and call);
  `tasks/mcp_discovery.rs` (boot + every 60s, 24h stale prune); `tool_discovery.rs`.
- **MCP** — tools declared **inline** in `crates/mcp/src/lib.rs`: `context_pack`,
  `get_layered_context`, `search`, `get_patterns`, `get_workspace_summary`,
  `match_pattern`, `get_pattern_for`, `update_session`, `find_workspaces`,
  `get_user_for_workspace`, `set_stance`, `use_workspace`, `pause_run`, `start_run`,
  `record_outcome`, … grouped by category: Codebase · Session · Memory · Library ·
  Pattern · Governance · Inference · Gateway.
- **MCP, designed — not built** (D61–D63). The names are **proposals**, chosen to sit beside
  `get_rules` and `get_workspace_summary`; none exists in `crates/mcp/src/lib.rs` or anywhere under
  `crates/`, `app/src` or `dojo/src` today.

  | Proposed tool | Input | Returns | Side effect | Daemon route it would call |
  |---|---|---|---|---|
  | `get_ticket` | `key` (e.g. `AUTH-412`); `session_id` | the ticket, tracker-agnostic: key, title, description, acceptance criteria, status, sprint, assignee, tracker and link; **plus its rules and resolved targets** (the `get_targets` payload) so one call starts the work | **records start of work** on the ticket and **links the session** to it | extends `GET /api/workspaces/{id}/tracker-tasks/{key}` (built: key, summary, issue type, status, current sprint) |
  | `get_targets` | optional `key`; repository from the cwd | the targets that apply here, each with its threshold, mode (`block` / `warn` / `off`) and the ladder rung it comes from (D48), plus the ticket's task-type and ticket rules | none | not built (no target table) |
  | `get_missed_targets` | optional `key`; the change (defaults to the working tree against the development branch, D18) | each missed target: its reading, threshold, mode — "target missed (blocking)" for `block` (D47) — and **what to fix** (the files, functions or edges behind the miss); `not measured` when the index is incomplete | none; a miss still standing is the dōjō's to accept (T3, D61) | not built |

  Key matching on the branch or prompt (built, `sensei.session_task_links`) remains the fallback for an
  agent that fetched the ticket elsewhere, e.g. `gh issue view` (D63).
- **Data** — `sensei.mcp_servers`, `sensei.mcp_tool_manifests`, `sensei.assistant_tools`,
  `activity.assistant_events`, `sensei.nodes` (`embedding vector(384)`, HNSW
  `nodes_embedding_hnsw`), `sensei.edges`. The work-delivery tools read `sensei.tracker_tasks` and
  write `sensei.session_task_links`; a fetch-made link needs a new `link_source` value (today
  `branch`, `prompt`, and `commit` reserved) and a start-of-work timestamp — neither exists.

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
- **Lead.** **The ticket fetch is the start of work** (D63). `get_ticket` records it once per session
  and ticket and links the session; a second fetch in the same session records nothing new. An unknown
  key is a not-found error, never an invented ticket.
- **Lead.** **The fetch is tracker-agnostic.** The tool's shape is the same for Jira, ADO and GitHub
  Issues; the daemon owns the tracker read (its mirror, or the dōjō's for an org workspace, D49).
- **Lead.** **Targets reach the assistant at the start of work** (D62), resolved by the T3 ladder,
  each with the rung it came from — the same answer the desktop's "What applies here" shows (D67).
- **Lead.** **A missed target comes with what to fix** (D61). The handoff never says "blocked" without
  naming the reading and the code behind it, and never claims a pass on a reading it could not make.

## Signals shown

| Element | Meaning | Worked example | Decision it drives / next step |
|---|---|---|---|
| Tool declaration | Playground MCP tree / detail pane | `name: "sensei.search"` | open the tool in the Playground and try it |
| `defaults` object | argument contract for the Playground form | `{ "query": "sensei.projects", "limit": 10 }` | run the tool with its defaults |
| Category group | Playground rail grouping | `Codebase` (8 tools) | none — candidate to cut (navigation grouping) |
| Third-party MCP row | Health L1 grid | `postgres` from `.mcp.json` | fix or remove a failing server in `.mcp.json` |
| `context_pack` result | pack handed to the assistant | `ContextPack { nodes, total_tokens, summary, omitted }` | check `omitted`; widen the query when a needed file was left out |
| `match_mode` | which arm matched | `fts` · `semantic` · `structural` · `hybrid` | none — candidate to cut (diagnostic only) |
| `overall_confidence` | collapsed score | `high` = one hit ≥ 0.8 combined | low → refine the query, or read the files directly |
| Token-efficiency delta | Insights chip | "token spend down 30% since ranking landed" | none — candidate to cut (a result to read; no choice is offered) |
| `get_ticket` result (proposed) | the ticket handed to the assistant | `AUTH-412 · "Rotate refresh tokens" · In Progress · Sprint 23 · 3 acceptance criteria` | the assistant plans against the acceptance criteria; a not-found key → check the key or sync the tracker |
| Start of work (proposed) | session linked by fetch | `session 4f2c… started AUTH-412 at 10:42 (fetch)` | none — candidate to cut (a record; T3's linkage counts read it) |
| `get_targets` result (proposed) | targets that apply here | `Cov. ≥ 80% on auth/** · block · from Repository auth-svc` | the assistant writes the tests the target needs before it is missed |
| `get_missed_targets` result (proposed) | missed targets and what to fix | `target missed (blocking): coverage 77% on auth/** — add tests for login.ts` | the assistant clears each listed item; what stands goes to a lead or admin in the dōjō (T3) |
| `not measured` (proposed) | a target with no reading | `zone of pain · not measured (index 62%)` | let the index finish, then ask again |

## Done gate

- Every tool in the tools surface declares its `inputSchema`; the Playground's
  form for it is that schema, as `/api/instruments/mcp-servers/{id}/tools` lists it.
- Third-party MCPs configured in Claude Code / Zed appear in `sensei.mcp_servers`
  within 60s of daemon start.
- Playground executes any tool round-trip without the user typing an argument; each
  call is logged with attribution.
- `context_pack` returns within `max_tokens` (±100) and skips content the assistant
  already saw this session.
- A conceptually-phrased `search` returns the right file without a substring match;
  the grep fallback fires on `none`.
- *(Designed, not built.)* `get_ticket` with a synced key returns the tracker-agnostic ticket with its
  rules and targets, links the session and records start of work; an unknown key returns not-found.
- *(Designed, not built.)* `get_missed_targets` after a change that drops coverage on a guarded path
  returns that target as "target missed (blocking)" with the files to fix.

```bash
id=$(curl -s localhost:7744/api/instruments/mcp-servers | jq -r '.servers[] | select(.mcp_key|test("(^|_)sensei$")) | .id')
curl -s "localhost:7744/api/instruments/mcp-servers/$id/tools" | jq '[.tools[] | select(.inputSchema == null)] | length'   # 0
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
- **`get_ticket` returns a placeholder ticket for an unknown key**, or a fetch does not link the
  session — fabricated data / D63 regressed.
- **`get_missed_targets` reports a pass on a reading it could not make**, or a miss with no "what to
  fix" — D61 regressed.
- **A tool calls a target a "gate"** — D47 vocabulary regressed ("gate" is relay's approval word).

## Scenarios

- [`scenarios/mcp-surface.feature`](scenarios/mcp-surface.feature) — the tool
  catalog, the defaults contract, dispatch, resolution and third-party discovery
  (pinned).
- [`scenarios/context-delivery.feature`](scenarios/context-delivery.feature) — pack
  assembly, scopes and `context_pack` routes (pinned + `NO TEST` for the unbuilt
  dedup and levels).
- [`scenarios/search.feature`](scenarios/search.feature) — hybrid search, ranking,
  fusion and the grep fallback (pinned).
- [`scenarios/work-delivery.feature`](scenarios/work-delivery.feature) — the ticket fetch and start of
  work, targets at start of work, and the missed-target handoff (`NO TEST`; designed, not built).

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
- **Work-delivery tools** — `get_ticket`, `get_targets`, `get_missed_targets` (proposed names) are
  not built; they wait on T3's target table and evaluation, a mirror that holds description,
  acceptance criteria and assignee (today `tracker_tasks` holds none of them), and a fetch
  `link_source` with a start-of-work time.
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
- [T3 Dōjō Workspace Planning](../../04-team/03-dojo-workspace-planning/README.md) — targets, the
  ladder, the missed-target handoff and acceptance. [plan/2026-11-03-parallel-lanes.md §0](../../../../plan/2026-11-03-parallel-lanes.md) — D61–D63.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md](../../../01-vision.md#the-core-loop).
- Crates: `crates/mcp/src/lib.rs` · `crates/senseid/src/{api/handlers/query.rs,
  api/handlers/mcp.rs, tasks/mcp_discovery.rs, tool_discovery.rs}`.
