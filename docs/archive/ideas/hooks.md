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

Two hook scripts, one per mode — same pattern as the binary naming convention (`senseid` / `senseid-dev`):

| Script | Installed by | Port | Fallback |
|--------|-------------|------|----------|
| `~/.claude/hooks/sensei-hook.ts` | `sensei plugin install` | 7744 | `~/.sensei/events.jsonl` |
| `~/.claude/hooks/sensei-hook-dev.ts` | `sensei-dev plugin install` | 7745 | `~/.sensei-dev/events.jsonl` |

```
Assistant (Claude Code / Cursor / Zed / ...)
    │
    ▼ hook fires (stdin: JSON payload)
    │
    ├── sensei-hook.ts          → POST http://localhost:7744/hook/event (release)
    │                              fallback: ~/.sensei/events.jsonl
    │
    └── sensei-hook-dev.ts      → POST http://localhost:7745/hook/event (dev)
                                   fallback: ~/.sensei-dev/events.jsonl
```

Claude Code fires both entries when both are registered in `settings.json`. Each script is single-port — no fan-out, no dev-port caching. Uninstalling one mode's hook never touches the other mode's entries.

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

## Hook Registration (Claude Code)

Claude Code has no CLI to register hooks — the only method is editing `~/.claude/settings.json` directly. `sensei plugin install` writes the release entries; `sensei plugin uninstall` removes only the release entries. The dev binary manages its own entries independently.

### Release only (`sensei plugin install`)

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

### Both installed (`sensei plugin install` + `sensei-dev plugin install`)

Each event type gets two entries — Claude Code fires both. Each posts to its own daemon; each uninstall removes only its own entries.

```json
{
  "hooks": {
    "SessionStart": [
      { "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook.ts" }] },
      { "hooks": [{ "type": "command", "command": "~/.claude/hooks/sensei-hook-dev.ts" }] }
    ]
  }
}
```

`sensei-dev plugin uninstall` removes only the `-dev` entries. `sensei plugin uninstall` removes only the release entries. Neither binary touches the other's registration.

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
