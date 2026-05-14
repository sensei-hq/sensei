# Logging & Diagnostics

When something goes wrong, Sensei helps you understand what happened, surface the relevant details, and report the issue if needed. Diagnostic logging runs continuously in the background, capturing a structured trace of every operation so you have the data when you need it.

---

## Bootstrap diagnostics

During install and startup, every bootstrap gate captures a structured trace: the command that ran, its exit code, stdout, stderr, and how long it took. If a gate fails, the error appears on the bootstrap screen with enough context to understand what went wrong. You don't need to check terminal output or hunt through system logs -- the trace is right there, expandable, with the full command output.

This matters most on first install, when multiple dependencies need to be detected, installed, or started. If PostgreSQL won't start or the daemon fails its health check, the trace tells you exactly which command failed and what it reported.

---

## Log viewer

Sensei has a built-in log viewer accessible from the View menu. It shows a timeline of events with structured data for each entry.

### Filtering

- **By level** -- errors, warnings, info, debug, trace
- **By component** -- bootstrap, daemon, gateway, indexer, sessions
- **By time** -- narrow to a window, jump to a timestamp

### Tail mode

Leave the viewer open and new entries stream in as they happen. Useful when you're investigating an issue in real time -- start an operation in your IDE and watch the corresponding events appear in the log.

### Search

Full-text search across log entries. Find every mention of a specific file, error code, or model name.

### Export

Export the current view (with filters applied) for sharing. The export includes system information and the filtered log entries in a format suitable for pasting into an issue or sending to someone helping you debug.

---

## Debug mode

By default, Sensei captures a full trace log but only surfaces errors and warnings in the UI. Debug mode increases what's visible.

### Toggling

Enable debug mode from settings or by passing `--debug` when launching from the command line. When active, Sensei shows:

- Toast notifications on gate failures with error details
- Verbose gateway routing decisions (which model was selected and why)
- Indexing pipeline progress at the per-file level
- Hook event details as they fire

### What changes

Debug mode doesn't change what Sensei captures -- the full trace is always being recorded. It changes what's surfaced to you. Think of it as adjusting the volume on information that's already flowing.

---

## Issue submission

When you hit a problem you can't resolve, Sensei packages everything needed to file a useful bug report.

### What gets collected

- System information: OS version, chip, RAM
- Sensei version: desktop app, daemon, CLI, MCP server
- Bootstrap trace: the full gate-by-gate log from the most recent startup
- Recent error logs: filtered to the relevant timeframe

### One-click submission

A submit button on the log viewer or bootstrap screen generates a GitHub issue template pre-filled with the diagnostic data. You add a description of what you were trying to do and submit. The issue lands in the Sensei repository with all the context a maintainer needs to investigate.

No code content, file paths, or project-specific information is included in the submission. Only system diagnostics and Sensei's own logs.

---

## Session traces

Every coding session Sensei observes produces a per-session event log. This trace captures the lifecycle of the session from start to finish.

### What's recorded

- **Hook events** -- every tool call, correction, and file edit that flowed through from your AI assistant
- **MCP calls** -- which tools were invoked, what they returned, how long they took
- **Gateway decisions** -- which models were called, fallback events, cost incurred
- **Timing** -- duration of each phase, latency breakdowns, total session time
- **Outcomes** -- whether the session was first-time-right, what corrections occurred, final metrics

### Viewing traces

Session traces are accessible from the observatory's session detail view. They're most useful when investigating why a session went poorly -- you can see exactly what happened, in what order, and how long each step took.

---

## Reference

- Implementation details: [design/07-logging.md](../design/07-logging.md)
