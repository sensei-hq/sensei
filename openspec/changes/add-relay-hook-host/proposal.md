# Change: Hook-captured sessions report their host (#512)

## Why
A live, hook-captured session records no host: `activity.sessions.host` is
filled only by transcript ingest, because the plugin hook payload carries no
environment. A Claude Code session running in Zed's or Cursor's terminal is
attributed to no editor until its transcript is ingested, and never when it
has none. Relay lanes already detect their host at register (MCP client name,
then `TERM_PROGRAM` / `CURSOR_TRACE_ID`); hook-captured sessions should use
the same signal.

## What Changes
- **Payload.** The plugin's `send_telemetry` enriches the hook payload with
  `term_program` (`$TERM_PROGRAM`) and `cursor_trace` (whether
  `$CURSOR_TRACE_ID` is set — presence, never the value), where `jq` is
  available. The daemon stays tolerant: absent keys mean no evidence.
- **One table, shared.** The env→host rules move from the MCP binary's
  `detect_host` into `sensei-bootstrap` (`host` module); both the MCP binary
  (client name first, then env) and the daemon (env only) use them, so the
  mapping cannot drift between the two.
- **Daemon fold.** `apply_to_session` maps the payload's env, keeps the
  proposal only when the seeded catalogue knows it as an editor, and fills
  `sessions.host` with `COALESCE` — a transcript's host is never cleared,
  and a later event with no host never clears either.
