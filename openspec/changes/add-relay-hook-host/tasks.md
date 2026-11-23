# Tasks: add-relay-hook-host (#512)

TDD: red test first for each item. No DDL change (only the `host` column
comment gains the hook source).

- [x] 1. Shared table: `host` module in sensei-bootstrap with the
      env→host rules + tests; MCP `detect_host` delegates (its tests stay
      green unchanged)
- [x] 2. Plugin: `term_program` + `cursor_trace` enrichment in
      `send_telemetry`; test-hooks.sh pins it where jq exists
- [x] 3. Daemon: map + catalogue-gate + COALESCE fill in
      `apply_to_session`; tests for zed/terminal/never-clear/no-row
- [x] 4. Gates green, commit, push
