---
name: Bootstrap Diagnostic Logging
description: Structured per-session trace logs, /logs viewer screen, GitHub issue submission, and shared app logger
date: 2026-05-01
status: approved
---

# Bootstrap Diagnostic Logging

## Core principle

**All sidecar check functions are pure.** Every function executes, returns data, and has no side effects — no logging, no file I/O, no global state. The command layer collects the returned data and decides what to do with it (write a session log, drive the health screen, emit events). This makes every check independently unit-testable without mocking.

Bootstrap checks run in parallel (one thread per gate). Results are collated, then written as a session log in one atomic operation.

## Overview

Every bootstrap session captures a structured trace log — one JSON file per session written to the OS application support directory. A `/logs` screen in the app lets users browse past sessions, inspect individual trace steps, and submit a pre-filled GitHub issue with anonymized diagnostic data.

---

## 1. Rust: BootstrapTrace struct

Each check or remediation step in the bootstrap pipeline produces a `BootstrapTrace`. The bootstrap crate accumulates traces and returns them alongside results.

```rust
pub struct BootstrapTrace {
    pub id:            String,         // uuid v4
    pub ts:            String,         // ISO 8601 wall-clock time
    pub action_type:   TraceAction,    // Check | Resolve | Instruct
    pub step:          String,         // snake_case step name, e.g. "postgres_port"
    pub desc:          String,         // human-readable label
    pub cmd:           String,         // command string run (or "tcp probe host:port")
    pub exit:          Option<i32>,    // process exit code (None for tcp probes)
    pub out:           String,         // stdout (trimmed)
    pub err:           String,         // stderr (trimmed)
    pub ms:            u64,            // wall-clock duration in milliseconds
    pub ok:            bool,           // did this step pass?
    pub fix_attempted: bool,           // did the bootstrap attempt a fix?
    pub fix_approach:  Option<String>, // command/strategy used to fix
    pub fix_ok:        Option<bool>,   // did the fix succeed?
}

pub enum TraceAction {
    Check,    // passive probe — binary present, port open, DB exists
    Resolve,  // active remediation — install, start service, create DB
    Instruct, // human action required — displayed as an instruction to the user
}
```

### Trace-capturing helper

A `run_traced(step, desc, cmd, args) -> BootstrapTrace` helper wraps every `Command::new()` call:

- Records wall-clock start time
- Runs the command, captures stdout/stderr/exit code
- Measures duration
- Returns a fully-populated `BootstrapTrace` with `ok = exit_code == 0`

TCP port probes use `probe_traced(step, desc, host, port)` which synthesises a fake `cmd` string (`tcp probe host:port`) and sets `exit` to `None`.

### Session accumulation

`check()` and `setup()` in each module return `(ComponentStatus, Vec<BootstrapTrace>)`. The Tauri sidecar collects all traces from all modules and bundles them into a `BootstrapSession`.

---

## 2. Session log files

### Location

`{app_data_dir}/sensei/logs/bootstrap/` — one JSON file per session.

- `app_data_dir` on macOS: `~/Library/Application Support/dev.sensei.desktop`
- Filename: `{session_id}.json` (e.g. `sess-a1b2c3d4.json`)
- Keep the last 20 session files; older ones are deleted on write.

### Session JSON shape

```json
{
  "id":          "sess-a1b2c3d4",
  "module":      "bootstrap",
  "started_at":  "2026-05-01T10:42:15.000Z",
  "app_version": "0.1.0",
  "system_info": {
    "os":        "macOS 15.3 Sequoia",
    "arch":      "arm64 (Apple M3)",
    "ram_gb":    16,
    "cpu_cores": 10
  },
  "outcome":     "success",
  "duration_ms": 3218,
  "traces": [ /* BootstrapTrace[] */ ]
}
```

`module` identifies which part of the app produced this session. Known values: `"bootstrap"` | `"wizard"` | `"projects"`. Additional modules can be added as the app grows — the log viewer groups by module and renders an unknown module with a fallback label.
```

`outcome` is derived: `"success"` if all traces pass, `"partial"` if some passed after a fix, `"failed"` if any remain failing.

### Tauri command

`get_bootstrap_logs() -> Vec<BootstrapSession>` reads all files from the log directory, sorted newest-first, and returns them to the frontend.

---

## 3. /logs screen (SvelteKit)

**Route:** `(app)/logs/+page.svelte`  
**Menu:** `View > Diagnostic Logs` (Tauri menu API)  
**Mockup:** `docs/mockups/logs-a.html`

### Layout

Two-panel layout:

- **Left sidebar (248 px):** sessions grouped first by date (Today / Yesterday / Apr 29…), then by module within each date. Date groups are collapsible — Today is expanded by default, older dates collapsed. Module sub-headers use kanji glyph + uppercase label. Session rows show time-only (date is the group header), outcome dot, `duration_ms`, step count, fix count. Clicking a row selects it. Ordering is reverse chronological.
- **Right panel:** trace table for the selected session — columns: step, action type badge, command, duration, result. Fixed `CURRENT` / session rows for visual grouping.

### State

```ts
// +page.ts
export async function load(): Promise<{ sessions: BootstrapSession[] }> {
  const sessions = await invoke<BootstrapSession[]>('get_bootstrap_logs');
  return { sessions };
}
```

The page component holds `selectedId` and `showPreview` as local state. No store needed — data is read-once.

### Report modal

Triggered by "Report this session ↗" button (solid dark). Rendered as a centered modal (84 vw, max 960 px, 90 vh).

**Two-column layout inside the modal:**

Left column — issue preview:
- Label: `ISSUE PREVIEW — ANONYMIZED`  
- Title field (read-only): `Bootstrap diagnostic — {os} · {arch} · v{app_version}`
- Body preview (monospace, scrollable): full markdown — system info, bootstrap trace table, fix details section (only included when stderr or fix_attempted present)

Right column (264 px):
- "Included in report" summary panel
- "Additional context" textarea
- Privacy note: `/Users/` replaced with `~/`
- Submit to GitHub ↗ (solid dark button) — opens pre-filled GitHub issue URL
- Copy markdown (outline button)

### Path anonymization

All strings in the issue body pass through `anonymize(s)`:
```ts
const anonymize = (s: string) => s.replace(/\/Users\/[^/]+\//g, '~/');
```

### GitHub issue URL

```ts
const url = `https://github.com/sensei-hq/app/issues/new?title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`;
```

Opened via Tauri `open()` shell plugin — no token or API key required.

---

## 4. Shared app logger

A module-scoped logger used across the entire app. Each module (bootstrap, wizard, projects, …) owns one active session at a time. Sessions are written to disk incrementally — crash-safe.

### Session lifecycle

- **Module opened** → `getModuleLogger('module')` starts a new session and returns a bound logger.
- **Module closed** (navigating away, component unmount, app quit) → the session is finalized: `outcome` derived from highest log level seen (`error` → failed, `warn` → partial, else success), `duration_ms` computed, file closed.
- **Only one active session per module at a time.** Calling `getModuleLogger('wizard')` while a wizard session is already open resumes it (same session ID). Calling `getModuleLogger('bootstrap')` does not close an open wizard session — modules are independent.
- **App quit** (`on_before_close` Tauri event) closes all open sessions.

### Log entry structure

```ts
interface LogEntry {
  id:      string;             // uuid v4
  ts:      string;             // ISO 8601
  level:   'info' | 'warn' | 'error';
  layer:   'ui' | 'api' | 'sidecar' | 'data_load';
  step:    string;             // action name, e.g. "preferences_save"
  msg:     string;             // human-readable description
  data?:   Record<string, unknown>;  // structured payload (cmd, status, ms, …)
  err?:    string;             // error message when level === 'error'
  stack?:  string;             // stack trace when available
}
```

**Layer meanings:**
- `ui` — component logic, event handlers, state transitions
- `api` — HTTP calls to the daemon (`/api/*`)
- `sidecar` — Tauri `invoke()` commands, IPC
- `data_load` — SvelteKit `load()` functions, initial data fetching

### Logger API

```ts
// Get (or create) the logger for a module. Returns the same instance
// if a session is already open for this module.
const logger = getModuleLogger('wizard');

// Standard log methods — each appends a LogEntry to the session file immediately
logger.info('api',       'preferences_save', 'Preferences saved',  { status: 200, ms: 58 });
logger.warn('api',       'scan_slow',        'Scan took >3s',       { ms: 3412 });
logger.error('sidecar',  'daemon_unreachable','Invoke failed',       new Error('connection refused'));

// Lifecycle — call from component's onDestroy / beforeNavigate
logger.close();
```

`logger.error()` accepts either a plain string or an `Error` object — if `Error`, `err` is set to `error.message` and `stack` to `error.stack`.

### Session file location and format

Path: `{app_data_dir}/sensei/logs/{module}/{session_id}.json`

Same top-level shape as bootstrap sessions, but `traces` contains `LogEntry[]`:

```json
{
  "id":          "sess-a1b2c3d4",
  "module":      "wizard",
  "started_at":  "2026-05-01T10:43:02.000Z",
  "app_version": "0.1.0",
  "system_info": { "os": "…", "arch": "…", "ram_gb": 16, "cpu_cores": 10 },
  "outcome":     "success",
  "duration_ms": 8140,
  "traces": [ /* LogEntry[] */ ]
}
```

Entries are appended to the file as they are logged (not buffered). The file is valid JSON — the writer maintains an open array, appending `,{…}` after each entry and writing the closing `]` on `close()`.

Keep the last 20 session files per module. Older files deleted on session start.

### Dev vs production

- **Dev** (`import.meta.env.DEV`): each `info`/`warn`/`error` call also prints to `console.debug` / `console.warn` / `console.error` with `[module:layer]` prefix.
- **Production**: silent — file writes only, no console output.
- No network calls, no external service.

### Data flow

```
UI logger  ──invoke──▶  Sidecar collector  ──write──▶  Disk (JSON files)
                               │
UI /logs page  ◀──invoke──  read from FS
```

The UI logger never touches the filesystem directly. Every log entry is forwarded to the sidecar via Tauri `invoke`, and the sidecar is the sole writer and reader.

### Sidecar: LogCollector (Rust)

`app/src-tauri/src/log_collector.rs` — a thread-safe state machine holding one open session file handle per module.

**Tauri commands exposed:**

| Command | Description |
|---------|-------------|
| `log_session_start(module, app_version, system_info) -> session_id` | Opens a new session file, returns the generated session ID |
| `log_entry(session_id, entry: LogEntry) -> ()` | Appends one entry to the open session. Fire-and-forget from the UI. |
| `log_session_end(session_id) -> ()` | Finalizes the session: derives `outcome`, writes `duration_ms`, closes the file handle |
| `get_log_sessions(module?) -> Vec<LogSession>` | Reads all session files (optionally filtered by module) from the log directory, returns newest-first |

`log_entry` writes immediately to disk (no buffering) — if the app crashes, all entries up to the crash are preserved.

File rotation: on `log_session_start`, if the module's log directory already has ≥ 20 files, the oldest are deleted before creating the new one.

---

## 5. Tauri menu item

Added to the app menu under `View`:

```rust
MenuItem::new("Diagnostic Logs", true, Some("CmdOrCtrl+Shift+L"), app_handle)
  .on_activate(|app| { app.emit("open-logs", ()).ok(); })
```

The frontend listens for `open-logs` event and navigates to `/logs`.

---

## 6. Scope summary

| Component | What changes |
|-----------|-------------|
| `daemon/crates/bootstrap/src/types.rs` | Add `BootstrapTrace`, `TraceAction`, `BootstrapSession` |
| `daemon/crates/bootstrap/src/util.rs` | Add `run_traced()`, `probe_traced()` helpers |
| `daemon/crates/bootstrap/src/database.rs` | Wrap all `Command::new()` calls with `run_traced()` |
| `daemon/crates/bootstrap/src/lib.rs` | `check()` and `setup()` return `Vec<BootstrapTrace>` |
| `app/src-tauri/src/commands/bootstrap.rs` | Collect traces, write session JSON, expose `get_bootstrap_logs` |
| `app/src-tauri/src/menu.rs` | Add `View > Diagnostic Logs` menu item |
| `app/src/routes/(app)/logs/+page.ts` | Load sessions via `invoke` |
| `app/src/routes/(app)/logs/+page.svelte` | Session list + trace table + report modal |
| `app/src-tauri/src/log_collector.rs` | `LogCollector` state, session file management, Tauri commands |
| `app/src/lib/logger.ts` | `getModuleLogger()` factory — sends entries to sidecar via invoke |

---

## 7. Testing

- **Unit:** `BootstrapTrace` struct, `TraceAction` serialization, session JSON round-trip, `anonymize()`, `LogEntry` struct, `LogCollector` file rotation (delete oldest when ≥ 20 files)
- **Integration:** `run_traced()` with a real command (`echo hello`), `LogCollector` start→append→end cycle on a temp dir, `get_log_sessions` reads and sorts correctly
- **Component:** `/logs` page with stub session data — verify trace table renders for bootstrap and wizard modules, modal opens, anonymization applied
- **E2E (browser mode):** Click "Report this session" → modal appears → markdown body contains "Fix details" section for failed traces
