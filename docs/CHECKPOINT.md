# Checkpoint — 2026-11-20 (relay open items — #455 slice 2/2)

**Slice:** #455 slice 2 — host detection at register (`add-relay-assistant-catalogue`, tasks 5.1-5.7).
**Branch:** `lane/relay-455-s2` (worktree `lanes/relay-open`), from origin/develop after slice 1 (PR #506).

## Done
- 5.1: client names and env values checked on this machine with a logging MCP probe (Claude Code
  `claude-code`, Zed `Zed`, VS Code `Visual Studio Code`, Terminal.app `Apple_Terminal`); recorded in design.md.
  Cursor, Windsurf and Kiro are not installed here; their published values stand.
- 5.2/5.3: `crates/mcp` `detect_host(client_name, env)` and `with_detected_host`; `main.rs` keeps
  `clientInfo.name` from `initialize`, reads the environment once, and register carries `host_detected`
  (a caller's own `host_detected` is dropped).
- 5.4-5.6: `relay::host::resolve_host`; register prefers a detected catalogue editor, falls back to the
  report, warns on a mismatch or an ignored detection, refuses either value over 64 characters. No DDL.
- 5.7: `app/e2e/tests/relay-host.spec.ts` green with relay-run and relay-catalogue (3 passed).
- 6.2: openspec validate --strict; lanes.md §4 describes the host and the picker.
- 6.3: follow-ups raised: #512 (hook-captured sessions' host), #513 (launch templates).

## Remaining
- PR to develop (Closes #455); CI green; merge.
- 6.1's full `make test-app-e2e` across every spec was not run here (the three relay specs were).

## Next command
`gh pr checks <pr> --watch`

## Open questions
- None.

## Known-broken
- None.
