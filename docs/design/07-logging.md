# Logging — Structured Tracing & Diagnostics

**See also:** [ideas/06](../ideas/06-logging.md)

## Overview

Sensei uses structured tracing for all diagnostic output. The daemon emits structured spans and events via the `tracing` crate. The desktop app captures per-session trace logs written as JSON files to disk. A log viewer screen in the app lets users browse sessions, inspect traces, toggle debug verbosity, and submit pre-filled GitHub issues with anonymized data.

---

## Architecture

### Daemon (Rust)

The daemon uses the `tracing` crate with structured spans and events. Each log entry carries:

- **Level** — `trace`, `debug`, `info`, `warn`, `error`
- **Component tag** — identifies the subsystem (`bootstrap`, `scan`, `api`, `mcp`, `gateway`)
- **Span context** — nested spans for request tracing (e.g., `api::scan::process_folder`)
- **Structured fields** — typed key-value pairs (duration_ms, step, exit_code, etc.)

Output is JSON-formatted and written to a log file at `~/.sensei/logs/daemon.log` (release) or `~/.sensei-dev/logs/daemon.log` (dev). The daemon does not log to stdout in production — only in dev mode when run interactively via `make daemon-dev`.

### Desktop app

The app uses a shared `LogCollector` in the Tauri sidecar. Each app module (bootstrap, wizard, projects, etc.) owns one active session at a time. Log entries are forwarded from the SvelteKit frontend to the sidecar via `invoke` and written immediately to disk — no buffering, crash-safe.

Session files live at `{app_data_dir}/sensei/logs/{module}/{session_id}.json`. Each module retains the last 20 session files; older ones are deleted on session start.

---

## Bootstrap diagnostics

Bootstrap checks run in parallel (one thread per gate). Every check function is pure — no logging, no file I/O, no global state. The function executes, returns data, and the command layer decides what to do with it.

### BootstrapTrace

Each check or remediation step produces a `BootstrapTrace`:

```rust
pub struct BootstrapTrace {
    pub id:            String,         // uuid v4
    pub ts:            String,         // ISO 8601
    pub action_type:   TraceAction,    // Check | Resolve | Instruct
    pub step:          String,         // snake_case step name, e.g. "postgres_port"
    pub desc:          String,         // human-readable label
    pub cmd:           String,         // command string or "tcp probe host:port"
    pub exit:          Option<i32>,    // process exit code (None for tcp probes)
    pub out:           String,         // stdout (trimmed)
    pub err:           String,         // stderr (trimmed)
    pub ms:            u64,            // wall-clock duration
    pub ok:            bool,           // did this step pass?
    pub fix_attempted: bool,
    pub fix_approach:  Option<String>,
    pub fix_ok:        Option<bool>,
}
```

`TraceAction` distinguishes passive probes (`Check`), active remediations (`Resolve`), and instructions that require human action (`Instruct`).

### Per-gate logging

Each bootstrap gate (Homebrew, PostgreSQL, Ollama, sensei components, database, daemon) accumulates its own trace vector. Gates run in parallel. Results are collated into a `BootstrapSession` and written atomically as a single JSON file.

### Session JSON

```json
{
  "id": "sess-a1b2c3d4",
  "module": "bootstrap",
  "started_at": "2026-05-01T10:42:15.000Z",
  "app_version": "0.2.2",
  "system_info": { "os": "macOS 15.3 Sequoia", "arch": "arm64 (Apple M3)", "ram_gb": 16, "cpu_cores": 10 },
  "outcome": "success",
  "duration_ms": 3218,
  "traces": []
}
```

`outcome` is derived: `success` if all traces pass, `partial` if some passed after a fix, `failed` if any remain failing.

### Log file location

macOS: `~/Library/Application Support/dev.sensei.desktop/sensei/logs/bootstrap/`

---

## Log viewer API

The Tauri sidecar exposes four commands for the log viewer:

| Command | Description |
|---------|-------------|
| `log_session_start(module, app_version, system_info)` | Opens a new session file, returns session ID |
| `log_entry(session_id, entry)` | Appends one entry. Fire-and-forget. |
| `log_session_end(session_id)` | Finalizes: derives outcome, writes duration, closes handle |
| `get_log_sessions(module?)` | Reads all session files (optionally by module), returns newest-first |

### Log viewer screen

**Route:** `(app)/logs/+page.svelte`
**Menu:** `View > Diagnostic Logs` (Cmd+Shift+L)

Two-panel layout:
- **Left sidebar (248px):** Sessions grouped by date (Today / Yesterday / older), then by module. Date groups are collapsible. Session rows show time, outcome dot, duration, step count.
- **Right panel:** Trace table for the selected session — step, action type badge, command, duration, result. Expandable rows for stdout/stderr.

---

## Debug mode

Debug mode increases logging verbosity across all components. It is toggled via the app settings or the CLI `--debug` flag.

When active:
- Daemon log level drops from `info` to `debug` (more span detail, query timings, MCP tool invocations)
- Bootstrap surfaces gate failures as toast notifications with error details
- App logger also prints to `console.debug` / `console.warn` / `console.error` with `[module:layer]` prefix

In production (debug off): file writes only, no console output, no network calls, no external service.

---

## Issue submission

The log viewer includes a "Report this session" button that generates a pre-filled GitHub issue.

### Report modal

Two-column layout:
- **Left:** Issue preview with title (`Bootstrap diagnostic -- {os} . {arch} . v{app_version}`) and markdown body containing system info, bootstrap trace table, and fix details (only when stderr or fix_attempted present).
- **Right:** "Included in report" summary, additional context textarea, privacy note, Submit and Copy buttons.

### Path anonymization

All strings in the issue body pass through:

```ts
const anonymize = (s: string) => s.replace(/\/Users\/[^/]+\//g, '~/');
```

### Submission

Opens a pre-filled GitHub issue URL via the Tauri shell plugin — no token or API key required:

```
https://github.com/sensei-hq/app/issues/new?title={title}&body={body}
```

### What the report collects

- OS version, architecture, CPU cores, RAM
- App version
- Bootstrap trace table (step, command, exit code, duration, pass/fail)
- Fix details section (stderr content, fix approach, fix outcome)
- User-provided additional context
