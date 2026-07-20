# Layer · mcp

> **Serves:** the *deliver* step of the [core loop](../vision.md#the-core-loop)
> — getting the right context to the assistant, first try. This is where FTR is
> won or lost in the moment.

## What it is

`crates/mcp` — an MCP (Model Context Protocol) server the assistant runs over
stdio. It is a **thin proxy dispatcher**: most tools forward to the daemon's
HTTP API on :7744; a few (infer/embed/consensus) call the gateway inline. The
assistant (Claude Code · Zed · Cursor) calls these tools mid-task.

```mermaid
flowchart LR
    ASSIST[AI assistant] -->|stdio| MCP[mcp server]
    MCP -->|HTTP :7744| D[daemon]
    D --> DATA[(sensei DB)]
    MCP -.->|inline| GW[gateway]
```

## The tool surface

Grouped by what the assistant needs:

| Group | Tools | State |
|---|---|---|
| **Code navigation** | `search`, `get_callers`, `get_callees`, `get_project_summary`, `get_project_conventions`, `get_duplicates`, `get_communities` | mostly **works** (clean post-#101); two bugs ↓ |
| **Patterns / rules** | `get_patterns`, `get_pattern_for`, `match_pattern`, `get_rules`, `get_commands` | `get_patterns` empty (tagger unrun) |
| **Context** | `get_layered_context`, (spec: `context_pack`) | layered context works; `context_pack` **unbuilt** |
| **Libraries** | `get_lib_docs`, `search_lib_docs`, `add_library` | works (text-match, not semantic) |
| **Inference** | `infer`, `embed`, `consensus`, `generate_image`, `gateway_status` | works |
| **Knowledge write** | `propose/save/promote_memory`, `record_outcome`, `accept/reject_proposal` | wired |
| **Session** | `create/update_session`, `log_event`, `update_phase`, `get_workflow_state` | wired (capture) |

## Known gaps (the deliver step is only ~80% real)

- **`search` is substring, not semantic** — plain `ILIKE` over an *embedded*
  corpus (157k nodes). The spec's hybrid semantic layer (`hybrid.rs`,
  `context_pack`, grep-fallback under `crates/mcp/src/tools/`) **does not exist**.
  This is the flagship differentiator — Phase 2 (open-issues G4).
- **`get_communities` returns empty** despite 158 real communities — a
  single-folder scoping bug (`folder_ids_for_project` takes the lowest-UUID leaf
  instead of aggregating all scope folders). Cheap fix — Phase 0/1 (G5a).
- **`get_patterns` returns empty** — `file_tags` has 45,871 rows, all tag-arrays
  empty; the framework-pattern tagger never ran (G5b).
- **Corpus is thin** — the plumbing works end-to-end; there's little captured
  knowledge yet for the assistant to receive (1 project rule). Fills as capture
  + promotion accrue.

## Discovery — a per-assistant trait

Tool discovery differs by assistant (Claude Code `~/.claude/mcp.json` + project
`.mcp.json`; Zed `context_servers`; Cursor `.cursor/mcp.json`). It is (being)
modelled as a `ToolDiscovery` trait with per-assistant impls feeding one unified
inventory (`assistant_tools` / `mcp_servers`).

## Design rationale (integration constraints)

- **REPO_PATH resolves in 3 steps** (transparent to the assistant — it never
  passes a project id): env `REPO_PATH` set at registration → process cwd → the
  first project in the registry (single-project fallback).
- **Error contract:** every tool distinguishes "no matches" (empty array) from
  "not indexed" (actionable message, e.g. "Run `sensei init`") from
  daemon-unreachable (a fix hint) — **never a silent empty**.
- **OTLP lives in the daemon, not the per-session MCP server** — because Claude
  Code starts one MCP server *per session*; N concurrent sessions would each try
  to bind the OTLP port, only the first wins and the rest crash before the MCP
  handshake, losing all tools. So the daemon owns OTLP; MCP servers
  `POST /otlp/register` on startup and the daemon time-window-correlates.
- **The MCP server stays coordinator-agnostic** (speaks standard MCP); a
  `CoordinatorAdapter` absorbs the four per-assistant concerns — where to register
  (config file), how to capture events (hooks/OTLP/poll/extension), where to
  install skills, and the project-context file (CLAUDE.md vs AGENTS.md vs
  `.cursorrules` — different name *and* content). Adding a coordinator = a new
  adapter only, no tool changes; the generic fallback is git-diff polling +
  `.sensei/context.md`.
- **Capability Registry** (design, deferred): features declare required data +
  source + `discard_when` (upstream issue) + a workaround that stays inert until
  upstream lands, then is cleaned; the UI degrades with "est." badges. See
  [decisions](../plan/decisions.md).

Intended contracts: [`../spec/pipeline/context-delivery.md`](../spec/pipeline/context-delivery.md),
[`../spec/pipeline/semantic-search.md`](../spec/pipeline/semantic-search.md),
[`../spec/pipeline/mcp-surface.md`](../spec/pipeline/mcp-surface.md).
