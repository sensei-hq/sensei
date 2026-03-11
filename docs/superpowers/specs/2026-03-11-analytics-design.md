# Analytics — Sub-project 1: Telemetry Collector + Analytics CLI

## Goal

Capture every Claude tool call in SQLite and expose it via `sensei stats` so teams can measure sensei's real-world impact, track which tools are used, and find missed opportunities where bash was used instead of a sensei tool.

## Architecture

### Overview

```
Claude hooks (PreToolUse / PostToolUse)
    │
    ▼  POST /event (100ms timeout)
sensei daemon  (Node.js, localhost:51789)
    │
    ├── SQLite  ~/.sensei/<uuid>/analytics.db   ← primary store
    └── JSONL   ~/.sensei/<uuid>/events.jsonl   ← fallback when daemon down
```

The daemon is a single Node.js process started by launchd (macOS) or systemd (Linux) at login. Hooks are shell scripts installed to `~/.claude/hooks/` by `sensei setup`.

When the daemon is not running, hooks write to JSONL (fire-and-forget, ≤100ms). On next daemon start, the JSONL is drained into SQLite.

### New Package

`packages/collector/` — standalone Node.js package containing:
- `daemon.ts` — HTTP server, SQLite writes, JSONL drain
- `stats.ts` — query engine for `sensei stats`
- `install.ts` — hook script generation and `~/.claude/settings.json` patching

---

## Event Schema

### `events` table

| Column | Type | Notes |
|--------|------|-------|
| id | INTEGER PRIMARY KEY |  |
| user_uuid | TEXT | from `~/.sensei/uuid` |
| session_id | TEXT | from `$CLAUDE_SESSION_ID` env var |
| seq | INTEGER | monotonic per session |
| ts | INTEGER | Unix ms |
| tool | TEXT | tool name (e.g. `search_index`, `Bash`) |
| phase | TEXT | `pre` or `post` |
| duration_ms | INTEGER | null for pre events |
| success | INTEGER | 1/0, null for pre events |
| input | TEXT | JSON-encoded tool input (truncated at 2 KB) |
| error | TEXT | error message on failure, null otherwise |
| project_path | TEXT | absolute path of active project |

### `projects` table

| Column | Type | Notes |
|--------|------|-------|
| path | TEXT PRIMARY KEY |  |
| first_seen | INTEGER | Unix ms |
| last_seen | INTEGER | Unix ms |

### `daily_stats` table (pre-aggregated)

| Column | Type |
|--------|------|
| date | TEXT (YYYY-MM-DD) |
| tool | TEXT |
| calls | INTEGER |
| successes | INTEGER |
| total_duration_ms | INTEGER |

---

## Collector Daemon

### HTTP API

| Endpoint | Method | Body | Response |
|----------|--------|------|----------|
| `/event` | POST | JSON event object | `{"ok":true}` |
| `/health` | GET | — | `{"ok":true,"uptime":N}` |

### Startup sequence

1. Read or generate UUID at `~/.sensei/uuid`
2. Open SQLite with `PRAGMA journal_mode=WAL`
3. Run migrations (create tables if missing)
4. Drain `events.jsonl` if present and non-empty
5. Start HTTP server on `localhost:51789`

### JSONL fallback

When the daemon is unavailable, hooks append a JSON line to `~/.sensei/<uuid>/events.jsonl`. The hook exits within 100ms regardless of daemon state (fire-and-forget with 100ms `curl` timeout).

### Daemon management

- **macOS:** launchd plist at `~/Library/LaunchAgents/com.sensei.collector.plist`
- **Linux:** systemd user unit at `~/.config/systemd/user/sensei-collector.service`
- Both installed by `sensei setup`

---

## Hook Scripts

Installed to `~/.claude/hooks/` by `sensei setup`:

**`pre-tool-use.sh`** — reads `$CLAUDE_TOOL_NAME` and `$CLAUDE_TOOL_INPUT`, posts to daemon with 100ms timeout, falls back to JSONL append.

**`post-tool-use.sh`** — reads `$CLAUDE_TOOL_NAME`, `$CLAUDE_TOOL_RESULT`, `$CLAUDE_TOOL_DURATION_MS`, `$CLAUDE_TOOL_SUCCESS`, posts to daemon, falls back to JSONL.

Both scripts are registered in `~/.claude/settings.json` under `hooks.PreToolUse` and `hooks.PostToolUse`.

---

## `sensei stats` CLI

### Default output (7-day)

```
sensei stats — last 7 days

Tool calls: 842
  search_index    341  (40%)   ✓ 98%  avg 120ms
  Bash            289  (34%)   ✓ 91%  avg 340ms
  Read            112  (13%)   ✓ 100% avg  45ms
  reindex_repo     58   (7%)   ✓ 97%  avg 8.2s
  ...

Sessions: 24 across 3 projects
```

### `--gaps` output

```
Missed opportunity report — last 7 days

Pattern                          Count   Suggested tool
───────────────────────────────  ─────   ─────────────────
grep -r "..." src/               47      search_index (semantic)
cat src/...                      31      Read (or get_file_context)
find . -name "*.ts"              18      Glob
```

Gaps are detected by pattern-matching bash commands against known sensei tool equivalents.

---

## Error Handling

| Failure | Behaviour |
|---------|-----------|
| Daemon not running | Hook falls back to JSONL; no error shown to user |
| JSONL write fails | Hook exits silently; event is lost |
| SQLite write fails | Daemon logs to stderr; returns `{"ok":false}` |
| Hook timeout (>100ms) | Hook process exits; event dropped or written to JSONL |
| Daemon drain fails | Daemon logs warning; continues serving new events |

All failures are silent from the user's perspective. Reliability of the underlying session is never compromised.

---

## Testing

- **Unit:** daemon HTTP handlers, SQLite writes, JSONL drain logic, stats query engine, gap detector
- **Integration:** full hook → daemon → SQLite round trip, JSONL fallback path
- **CLI:** `sensei stats` output format and flag combinations
- **No mocking of SQLite:** tests use in-memory SQLite or a temp file

---

## File Map

```
packages/collector/
  src/
    daemon.ts         HTTP server + SQLite writes + JSONL drain
    stats.ts          Query engine (used by sensei stats)
    install.ts        Hook script generation + settings.json patching
    uuid.ts           UUID read/generate at ~/.sensei/uuid
    schema.ts         SQLite migrations
    gaps.ts           Bash pattern → sensei tool suggestion logic
  tests/
    daemon.test.ts
    stats.test.ts
    install.test.ts
    gaps.test.ts
packages/cli/src/commands/
  stats.ts            sensei stats command (calls packages/collector stats)
  setup.ts            sensei setup command (calls packages/collector install)
```
