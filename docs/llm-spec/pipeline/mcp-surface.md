# 具 · Pipeline · MCP tool surface

**Owner files:**
- Tool declarations: `crates/mcp/src/tools/`
- Server: `crates/mcp/src/server.rs`
- Discovery of third-party MCPs: `crates/senseid/src/db/pg_store.rs` (`sensei.mcp_servers`)
- Playground execution: `crates/senseid/src/api/handlers/mcp.rs::call`

## Purpose

The MCP surface is **how sensei's brain becomes usable**. The daemon
knows what your code looks like, what your sessions did, what
patterns are forming — but that knowledge is only valuable when the
assistant can reach for it during a session. The MCP surface is the
set of tools the assistant calls to do exactly that.

This pipeline governs:

1. **Sensei's own tool surface** — the ~40 tools in
   `crates/mcp/src/tools/` (`search`, `get_callers`, `get_project_summary`,
   `add_library`, `get_memories`, `save_memory`, …).
2. **Third-party MCP discovery** — Postgres MCP, Stripe MCP, GitHub
   MCP, etc. Discovered from the developer's assistant config
   (`.mcp.json`, `.claude/settings.json`) and tracked in
   `sensei.mcp_servers`.
3. **Tool defaults contract** — every tool declares sensible default
   arguments; the UI never needs to guess.
4. **Category taxonomy** — tools are categorised so the Playground
   MCP tree groups them meaningfully.

Kanji is 具 — *instrument*.

## Data invariants

### Sensei tools

- Every tool declares:
  ```rust
  Tool {
      name: "sensei.search",
      description: "…",
      input_schema: json_schema!({ query: string, limit?: number }),
      defaults: json!({ "query": "sensei.projects", "limit": 10 }),
      category: ToolCategory::Codebase,
      handler: handle_search,
  }
  ```
- `defaults` is the **contract**. The Playground argument form
  populates from this; the UI does not compute defaults itself
  (this was the reported UX bug where `repoId` fields came back
  empty). See (memory: feedback_no_command_guessing).
- Categories used today:
  - `Codebase` — `search`, `get_callers`, `get_callees`,
    `get_project_summary`, `get_duplicates`, `get_communities`,
    `get_project_conventions`, `get_layered_context`
  - `Session` — `create_session`, `update_session`,
    `update_phase`, `get_workflow_state`, `record_outcome`,
    `log_event`
  - `Memory` — `get_memories`, `save_memory`, `propose_memory`,
    `promote_memory`, `accept_proposal`, `reject_proposal`
  - `Library` — `get_lib_docs`, `search_lib_docs`, `add_library`
  - `Pattern` — `get_patterns`, `get_pattern_for`, `match_pattern`
  - `Governance` — `get_rules`
  - `Inference` — `infer`, `embed`, `consensus`, `generate_image`
  - `Gateway` — `gateway_status`
- **Every tool logs its call** — even ones invoked from the
  Playground. Attribution differs: assistant call → `session_id`;
  Playground call → `user_id` with a `playground=true` tag.
  Both are captured to `activity.assistant_events` so
  [[pipeline/signals]] can distinguish assistant patterns from
  user-driven exploration.

### Third-party MCP discovery

- `sensei.mcp_servers` — one row per discovered MCP:
  - `id` uuid, `acp_family` (`claude | cursor | zed | …`),
    `mcp_key` text (e.g. `postgres`), `scope` (`global | project`),
    `project_id` uuid nullable, `command`, `args`, `env`,
    `config_source` (path to the config file it was discovered
    from), `enabled` bool, `connection_state`
    (`connected | error | not_started`), `last_error` text,
    `last_seen_at` timestamptz, `discovered_at` timestamptz.
- Discovery pass runs on daemon boot and every 60s.
- Stale rows (`last_seen_at` older than 24h) are pruned.
- The Health L1 grid (see
  [[screen/observatory-instruments-health]]) reads from this
  table plus `sensei.tool_usage_stats` for `share_invoked`.

### Argument defaults — how they're computed

Defaults are declared per tool, then rewritten server-side with
context:

- Path fields like `repoId`, `project_id`, `folder_id` — default
  to the active project or the first-listed project. Never empty.
- Query fields — default to a known-good example
  (`"sensei.projects"` for `search`; `"main"` for a session name
  filter).
- Enum fields — default to the most common value.
- Nullable fields — omitted from defaults (the UI hides them by
  default; user can add).

This is the daemon's job. The Playground UI reads whatever
`defaults` came back and never invents its own.

## Signals produced

| Signal | Consumer |
|---|---|
| Tool declarations | Playground MCP tree, tool detail pane |
| Third-party MCP list | Health L1 grid |
| Category-grouped tool tree | Playground rail groups |
| Every tool call | `activity.assistant_events` → analyzer signals |
| Failed tool call | Signal derivation warn/opportunity path |

## Done gate

- Every tool in `crates/mcp/src/tools/` declares a non-empty
  `defaults` object appropriate to its schema.
- `GET /api/mcp/tools?mcp=sensei` returns all sensei tools with
  the categories and defaults matching the declarations.
- Third-party MCPs configured in Claude Code / Zed appear in
  `sensei.mcp_servers` within 60s of daemon start.
- Playground can execute any tool round-trip without the user
  having to type an argument.
- Playground-originated calls tag `playground=true` in
  `activity.assistant_events` so they don't corrupt
  assistant-behaviour signals.
- Failed calls surface a specific daemon error (not a swallowed
  "something went wrong").

Optional check:
```
curl -s 'http://localhost:7744/api/mcp/tools?mcp=sensei' \
  | jq '[.tools[] | {name, has_defaults: (.defaults | length > 0)}] | group_by(.has_defaults) | map({has_defaults: .[0].has_defaults, n: length})'
# expected: every tool has_defaults: true

psql -A -t -c "select acp_family, count(*) from sensei.mcp_servers
                where enabled and last_seen_at > now() - interval '1 hour'
                group by acp_family" -d sensei
```

## Wrong gate

- **A tool's `defaults` is `{}`.** Declaration doesn't populate
  the field — the exact bug that shipped and caused every
  Playground query to return empty.
- **Playground call succeeds but doesn't log to
  `assistant_events`.** Attribution missing; the user's own
  exploration would then feed the signal derivation as if it were
  an assistant pattern.
- **Third-party MCP appears in `sensei.mcp_servers` but the
  Health L1 grid shows only sensei.** The observatory reader is
  filtering to `family = 'sensei'` incorrectly.
- **`get_callers` returns empty and the fallback path (grep) isn't
  triggered.** Call-graph edges aren't fully populated — mask the
  empty result behind an obvious "coverage gap" hint rather than
  pretending there are no callers.
- **A newly added tool doesn't appear in the Playground tree
  after daemon restart.** Tool registration is compile-time; the
  build must include it.
- **Categorisation collapse.** All sensei tools land under
  `Codebase` because the category enum default is Codebase.
  Every new tool must explicitly pick.

## Related

- [[pipeline/capture]] — activity attribution + adapter architecture
- [[pipeline/signals]] — consumes `tool_usage_stats`
- [[pipeline/memory]] — `get_memories` / `save_memory` / promote
- [[pipeline/libraries]] — `add_library` / `get_lib_docs` /
  `search_lib_docs`
- [[screen/observatory-instruments-playground]] — Playground UI
- [[screen/observatory-instruments-health]] — L1 MCP grid consumer
- (memory: project_p2_sweep_2026_07) (memory) — recent tool-surface fixes
- (memory: feedback_no_command_guessing) — defaults contract
