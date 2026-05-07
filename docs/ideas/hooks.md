---
title: Hook Event Architecture
description: How sensei captures events from AI coding assistants via hooks
date: 2026-05-07
status: implemented
---

# Hook Event Architecture

## Purpose

Hooks give sensei visibility into AI assistant activity: which tools are called, how sessions begin and end, what prompts are submitted. This data feeds session analytics, FTR tracking, and pattern detection.

Hooks are assistant-agnostic. Claude Code is the first integration; the architecture supports Cursor, Zed, Kiro, and others as they expose similar event systems.

---

## Multi-Assistant Support

### Why it matters

Each assistant family has its own hook system:

| Assistant | Hook mechanism | Notes |
|-----------|---------------|-------|
| Claude Code | `~/.claude/settings.json` hooks | SessionStart, PreToolUse, PostToolUse, Stop, etc. |
| Cursor | Rules / `.cursor/settings.json` | Subset of events |
| Zed | Assistant panel extensions | Limited today |
| Kiro | Hooks config | Emerging |
| Opencode | Plugin system | Planned |

Each assistant's hook script is a thin wrapper that:
1. Reads the event payload from stdin
2. Injects `assistant_family` (identifies the source)
3. Injects `event_type` (normalized alias for the event name field)
4. POSTs to sensei daemon on the appropriate port(s)
5. Falls back to a local JSONL file if the daemon is down

### `assistant_family` enum

Defined in `database/ddl/enum/sensei/assistant_family.ddl`:

```
claude | cursor | zed | continue | codex | aider | opencode | kiro
```

Every hook event row in `activity.hook_events` carries this identifier. Queries can filter by family, compare behavior across assistants, or aggregate across all.

---

## Event Capture Pipeline

```
Assistant (Claude Code / Cursor / Zed / ...)
    │
    ▼ hook fires (stdin: JSON payload)
sensei-hook.ts (or equivalent per-assistant script)
    │  enriches payload:
    │    assistant_family: "claude"
    │    event_type: <hook_event_name>   ← normalized column alias
    │
    ├── POST http://localhost:7744/hook/event   (release daemon)
    │
    └── POST http://localhost:7745/hook/event   (dev daemon, if active)
              │
              └── fallback: append to ~/.sensei/events.jsonl
```

### Daemon ingestion

`POST /hook/event` always returns `200 OK`. Hook scripts must not block; errors are discarded silently. The handler in `crates/senseid/src/api/handlers/sessions.rs::ingest_hook_event` extracts:

| Field extracted | Source in payload |
|----------------|-------------------|
| `session_id`   | `payload.session_id` |
| `assistant_family` | `payload.assistant_family` (default: `"claude"`) |
| `event_type`   | `payload.hook_event_name` |
| `tool_name`    | `payload.tool_name` |
| `cwd`          | `payload.cwd` |
| `ts`           | server clock (ms) |
| `success`      | `exit_code == 0` (PostToolUse only) |
| `payload`      | full JSON blob |

---

## Dual Daemon Fan-out (Dev + Release)

When developing sensei itself, the dev daemon (port 7745, `~/.sensei-dev/`) runs alongside the release daemon (port 7744, `~/.sensei/`). Both receive hook events so development has real data.

### Dev port caching

Probing the dev port on every hook call would add latency for non-developers. The hook script uses a file-based cache at `~/.sensei/dev-port.cache` with a 30-second TTL:

```
active 1746600000000      ← "active" or "inactive", followed by epoch ms
```

- Cache hit (< 30s): use cached status, no probe
- Cache miss: HTTP GET `http://localhost:7745/health` with 100ms timeout → update cache

Non-developers never see the probe after the first miss (inactive gets cached for 30s, re-checked every 30s).

### Fan-out logic

```typescript
const endpoints = devActive ? [PROD_URL, DEV_URL] : [PROD_URL];
const results = await Promise.allSettled(endpoints.map(url => fetch(url, { body: enriched })));
// Fall back to file only if PROD (7744) fails — dev failure is non-critical
if (results[0].status === "rejected") {
  appendFileSync("~/.sensei/events.jsonl", enriched + "\n");
}
```

---

## Hook Registration (Claude Code)

Claude Code has no CLI to register hooks — the only method is editing `~/.claude/settings.json` directly. The `sensei plugin install` command writes the hooks block; `sensei plugin uninstall` removes it cleanly (without touching other hooks).

Example registration for all event types:

```json
{
  "hooks": {
    "SessionStart":        [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "InstructionsLoaded":  [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "UserPromptSubmit":    [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "PreToolUse":          [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "PostToolUse":         [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "Stop":                [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "SubagentStart":       [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "SubagentStop":        [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "Notification":        [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "PreCompact":          [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }],
    "PostCompact":         [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] }]
  }
}
```

---

## Database Schema

Table: `activity.hook_events`

| Column | Type | Notes |
|--------|------|-------|
| `id` | `bigserial` | Append-only high-write table; sequential inserts avoid B-tree splits |
| `session_id` | `text` | Assistant's string session ID — not a DB UUID |
| `assistant_family` | `sensei.assistant_family` | Source assistant; default `'claude'` |
| `event_type` | `text` | Normalized event name (SessionStart, PreToolUse, etc.) |
| `tool_name` | `text` | Tool name for Pre/PostToolUse; null otherwise |
| `cwd` | `text` | Working directory at event time |
| `ts` | `bigint` | Server-side millisecond timestamp |
| `success` | `boolean` | `true` if exit_code==0 (PostToolUse only); null otherwise |
| `payload` | `jsonb` | Complete hook payload from stdin |
| `created_at` | `timestamptz` | DB insert time |

Indexes: `(session_id, created_at desc)`, `(event_type, created_at desc)`, `(assistant_family, created_at desc)`, `(created_at desc)`.

`bigserial` over `uuid` for the PK: hook events are an append-only high-volume stream. Sequential bigserial inserts avoid random B-tree page splits that uuid PKs cause. 8 bytes vs 16 bytes per row also matters at scale.

---

## JSONL Import (Historical Data)

Historical events from `~/.sensei/events.jsonl` can be bulk-imported using the staging table:

```bash
# 1. Load JSONL into staging (new format with event_type field)
psql -d sensei_dev -c "
  COPY staging.hook_events(session_id, assistant_family, event_type, tool_name, cwd, ts, payload)
  FROM '/Users/you/.sensei/events.jsonl'
  (FORMAT text);
"

# 2. Transform into final table
psql -d sensei_dev -c "CALL staging.import_hook_events();"

# 3. Clear staging
psql -d sensei_dev -c "TRUNCATE staging.hook_events;"
```

`staging.normalize_assistant_family()` handles free-form strings ("Claude Code", "claude-code") and maps them to the enum.

`assistant_family` is a `sensei.assistant_family` enum in the final table. The staging table (`staging.hook_events`) stores it as `text` for easy loading. `dbd`'s `import_jsonb_to_table` (updated to handle custom PostgreSQL types) loads JSONL into staging directly. The `import_hook_events` procedure then casts `text → sensei.assistant_family` during the final insert.

---

## Future: Other Assistant Families

Adding hooks for a new assistant (e.g., Cursor) requires:

1. Write a hook script (e.g., `~/.cursor/hooks/sensei-hook.js`) that:
   - Reads the Cursor event payload from stdin
   - Enriches with `assistant_family: "cursor"` and `event_type: <cursor_event_name>`
   - POSTs to `http://localhost:7744/hook/event`

2. Register the script in Cursor's hook configuration

3. No daemon or DB changes needed — the `assistant_family` column already accommodates all known families

The daemon endpoint is intentionally assistant-agnostic: it accepts any JSON payload and stores `assistant_family` from the payload field (defaulting to `"claude"` if absent).
