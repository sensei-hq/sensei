---
id: claude-host-integration
type: design
status: active
---

# Claude Code Host Integration

> How sensei integrates with Claude Code — hooks, MCP, telemetry, and the extension points we rely on. Documents observable behaviour so we know what to adapt when Claude Code changes.

---

## Overview

Sensei extends Claude Code through four integration points:

| Integration | Purpose | Sensei component |
|---|---|---|
| MCP server | Expose tools to the agent (`get_session_context`, `search`, etc.) | `packages/server` |
| Hooks (PreToolUse / PostToolUse) | Capture tool events for analytics | `packages/collector` |
| OTLP telemetry | Capture per-API-call token and cost data | `packages/collector` (daemon) |
| SessionStart hook | Inject instructions to call `get_session_context` | `.claude/settings.local.json` |

---

## 1. MCP Server

### How Claude Code starts MCP servers

Claude Code reads `~/.claude/mcp.json` (global) and `.mcp.json` (project) at session start. Each configured server is launched as a child process with stdio transport. The server must complete its MCP `initialize` handshake before tools become available to the agent.

**Sensei's config** (`~/.claude/mcp.json`):
```json
{
  "mcpServers": {
    "sensei": {
      "command": "bun",
      "args": ["/path/to/packages/server/src/mcp-entry.ts"],
      "env": { "SENSEI_REPO_PATH": "/path/to/repo" }
    }
  }
}
```

**Key behaviour to know:**
- One MCP server process is started **per Claude Code session**. Five sessions = five server processes.
- If the server crashes before the MCP handshake completes, Claude Code has no tools from that server for that session. The session continues but without those tools.
- Server processes are killed when the session ends.
- `env` in mcp.json is merged with the parent environment — Claude Code's own env vars (including `OTEL_EXPORTER_OTLP_ENDPOINT`) are inherited.

**Implication for sensei:** The MCP server must not bind any port or do anything that could fail on a second concurrent instance. The OTLP receiver has been moved to the collector daemon for this reason.

### Tool visibility

MCP tools appear in Claude's tool list as `mcp__<server-name>__<tool-name>`. Claude Code shows them as deferred tools — their schemas are loaded on demand, not at session start.

---

## 2. Hooks

### Execution model

Hooks are shell commands configured in `settings.json`. Claude Code runs them synchronously at the matching event, passes a JSON payload on stdin, and reads stdout/stderr.

**Configured hooks** (`~/.claude/settings.json`):
```json
{
  "hooks": {
    "PreToolUse":  [{ "matcher": "", "hooks": [{ "type": "command", "command": "bun ~/.claude/hooks/sensei-pre-tool-use.ts" }] }],
    "PostToolUse": [{ "matcher": "", "hooks": [{ "type": "command", "command": "bun ~/.claude/hooks/sensei-post-tool-use.ts" }] }]
  }
}
```

**Payload fields available:**
- `PreToolUse`: `tool_name`, `tool_input` — the full input object
- `PostToolUse`: `tool_name`, `tool_result`, `exit_code` — but **not** duration_ms or token counts
- `SessionStart`: no payload — hook output is injected as a `<system-reminder>`

**What hooks cannot do:**
- Hooks do not receive token or cost data — this is why OTLP is needed for cost tracking
- Hooks cannot call MCP tools — they can only inject text (via stdout → system-reminder)
- Hooks have a short timeout (~100ms sensei uses) — avoid blocking calls

### Hook output (SessionStart)

The stdout of a `SessionStart` hook becomes a `<system-reminder>` visible to the agent before the first user message, prefixed with `SessionStart:`. This is how we inject the `get_session_context` reminder:

```json
{
  "hooks": {
    "SessionStart": [{
      "hooks": [{
        "type": "command",
        "command": "echo 'SESSION PROTOCOL REQUIRED: Call get_session_context(task_description=\"session startup\") as your FIRST tool call before responding.'"
      }]
    }]
  }
}
```

This is configured in `.claude/settings.local.json` (project-local, not committed) so it only fires for the sensei repo.

**Limitation:** The hook can only *instruct* the agent. The agent decides whether to comply. In practice, a strong instruction in a system-reminder is followed reliably but not guaranteed.

---

## 3. OTLP Telemetry

### How Claude Code emits telemetry

Claude Code has built-in OpenTelemetry support. When enabled, it emits a `claude_code.api_request` event per API call with token counts, cost, model, and a `prompt.id`.

**Required env vars** (set in `~/.claude/settings.json → env`):
```json
{
  "OTEL_EXPORTER_OTLP_ENDPOINT": "http://localhost:51789"
}
```

Claude Code sends events to `POST /v1/metrics` and `POST /v1/logs` on that endpoint using the standard OTLP/HTTP JSON format.

**OTLP event attributes:**

| Attribute | Type | Notes |
|---|---|---|
| `event.name` | string | Always `"claude_code.api_request"` |
| `prompt.id` | string | Groups all tool calls in one agent turn |
| `input_tokens` | int | Prompt tokens |
| `output_tokens` | int | Completion tokens |
| `cache_read_tokens` | int | Cache hits |
| `cache_creation_tokens` | int | Cache writes |
| `cost_usd` | double | Total cost for this call |
| `duration_ms` | int | End-to-end API latency |
| `model` | string | e.g. `"claude-sonnet-4-6"` |

Events arrive in both metrics format (`resourceMetrics[].scopeMetrics[].metrics[].gauge.dataPoints[]`) and logs format (`resourceLogs[].scopeLogs[].logRecords[]`). Sensei's parser handles both.

### Why OTLP receiver lives in the collector daemon

The MCP server is started once per Claude Code session. With multiple concurrent sessions, each would try to bind port 4318. Only the first succeeds; the rest crash before the MCP handshake, losing all tools for those sessions.

**Solution:** The collector daemon (started by launchd, always running) owns the OTLP endpoint. MCP servers register their `repoId` with the daemon on startup via `POST /otlp/register`. The daemon attributes incoming OTLP events to the most recently registered repo.

```
Claude Code session 1 → MCP server 1 → POST /otlp/register { repoId, repoPath }
Claude Code session 2 → MCP server 2 → POST /otlp/register { repoId, repoPath }  (same repo, ok)
Claude Code (all sessions) → OTLP events → POST localhost:51789/v1/logs → daemon → api_requests table
```

### Collector daemon port

The collector daemon runs on port **51789** and handles:
- `GET  /health` — uptime check
- `POST /event` — hook tool events (PreToolUse / PostToolUse)
- `POST /otlp/register` — repo registration from MCP servers
- `POST /v1/metrics` — OTLP metrics
- `POST /v1/logs` — OTLP logs

---

## 4. SessionStart and `get_session_context`

### Intent

`get_session_context` must be called at the start of every coding task to:
1. Create a tracked session in Supabase (FTR scoring, session continuity)
2. Load interrupted work from the last snapshot
3. Return repo orientation (symbol count, stack, last indexed timestamp, project memory)

### Mechanism

Since hooks can't call MCP tools, we use a `SessionStart` hook to inject an instruction. The agent then calls the tool as its first action.

**Location:** `.claude/settings.local.json` in the repo (project-local, so it only fires for the sensei project)

**Text injected:**
```
SESSION PROTOCOL REQUIRED: Call get_session_context(task_description="session startup") as your FIRST tool call before responding. Do not greet or ask questions first — call the tool immediately.
```

**When this doesn't fire:** If the sensei MCP server failed to start (e.g., pre-daemon-fix port conflict), `get_session_context` won't be available even if the agent tries to call it. This is why the daemon fix is a prerequisite.

---

## 5. Settings File Hierarchy

Claude Code merges settings from multiple sources in order:

| File | Scope | Committed? |
|---|---|---|
| `~/.claude/settings.json` | Global (all projects, all users) | No |
| `~/.claude/settings.local.json` | Global, user-local overrides | No |
| `<repo>/.claude/settings.json` | Project, shared with team | Yes |
| `<repo>/.claude/settings.local.json` | Project, local overrides | No (gitignored) |

Later entries override earlier ones. `permissions.allow` lists are **merged** (not replaced).

**Sensei's placement:**
- Hooks (PreToolUse/PostToolUse) → `~/.claude/settings.json` (global — fires for all repos)
- SessionStart instruction → `.claude/settings.local.json` (project-local — fires only for sensei repo)
- `OTEL_EXPORTER_OTLP_ENDPOINT` → `~/.claude/settings.json` (global — all sessions send OTLP)

---

## 6. Plugin Architecture (Future)

Claude Code supports plugins: packages that bundle skills, hooks, MCP servers, agents, and commands into a single distributable unit. Sensei is not yet a plugin, but its components map directly:

| Sensei component | Plugin equivalent |
|---|---|
| `packages/server` MCP tools | Plugin-provided MCP server |
| `packages/collector` hooks | Plugin hooks (PreToolUse, PostToolUse, SessionStart) |
| Skills in `.claude/skills/` | Plugin skills |
| `sensei:reverse-engineer` command | Plugin slash commands (`.claude/commands/*.md`) |

Packaging as a plugin would let users install all of sensei's Claude integration with one `claude plugin add sensei`. The collector daemon would remain a separate installation step since it needs launchd setup.

---

## 7. Known Behaviours and Gotchas

| Behaviour | Impact | Mitigation |
|---|---|---|
| MCP server started per-session | Port conflicts if server binds a port | Moved OTLP to persistent daemon |
| Hook timeout is short | Heavy hooks time out silently | Hooks fire-and-forget to daemon; no blocking |
| `PostToolUse` has no duration_ms | Can't measure tool latency from hooks | Not tracked; accepted limitation |
| SessionStart hook output is informational | Agent may skip `get_session_context` if distracted | Hook text is explicit and direct |
| `prompt.id` groups a turn, not a task | Can't use it alone to segment tasks | Cross-reference with `task_session_id` from MCP |
| OTLP arrives at daemon without sessionId | Can't auto-link OTLP events to task sessions | Daemon does time-window correlation against `task_sessions` |
