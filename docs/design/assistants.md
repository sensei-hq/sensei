---
type: design
---

# Assistants — module

Behind-the-scenes design for the assistant registration in
[Configuration](../features/02-config.md) (You → Assistants) and the capture
pipeline that feeds [Observatory](../features/03-observatory.md). The feature
docs say what the user sees; this says how connect + observe actually work.

## Registration — the Assistant trait

- Crate: `crates/senseid/src/assistants/` — `trait_def.rs` defines `Assistant`
  (`detect` · `configure` · `remove` · `upgrade`).
- `claude_code.rs` — bespoke impl: runs `claude plugin install`, registers the
  sensei plugin (MCP + hooks + skills + commands + agents as one unit).
  `upgrade()` re-runs `claude plugin update sensei` because Claude Code copies
  the plugin out at install time.
- `mcp_file.rs` — `McpFileAssistant`, a generic JSON-config-file impl covering
  Claude Desktop, Cursor, Windsurf, Zed, Kiro, VS Code, OpenCode. Detection by
  app bundle / bin / home path; two `McpEntryFormat`s (`Standard` vs
  `OpenCode`). These assistants get the MCP entry only — no hooks/skills, since
  they don't share Claude Code's plugin format.
- `health.rs` / `watchdog.rs` — `AdapterCheck`/`AdapterResolveReport`; an hourly
  watchdog re-verifies each configured assistant is still healthy (see
  `project_capture_watchdog` — design gap: skips unconfigured assistants, so a
  wipe on one can go unnoticed).
- This is the per-assistant side of the `CoordinatorAdapter` concept in
  [`../architecture/mcp.md`](../architecture/mcp.md) — one adapter absorbs
  where-to-register / how-to-capture / where-to-install-skills / which
  project-context file (CLAUDE.md vs AGENTS.md vs `.cursorrules`).

## MCP context delivery

- Crate: `crates/mcp` (`src/lib.rs`) — a thin stdio proxy; most tools forward to
  the daemon's HTTP API on `:7744`, a few (`infer`/`embed`/`consensus`) call the
  gateway inline. Full tool surface + gaps: [`../architecture/mcp.md`](../architecture/mcp.md).
- `get_rules` (line ~134) and `get_layered_context` (line ~150) are dispatched
  directly in `lib.rs`; `get_patterns` is a daemon alias — `map_daemon_tool`
  rewrites it to `get_file_tags` (line ~665, tool renamed `pattern`→`tag`).
- `context_pack` is declared in the tool surface (line ~339) but its hybrid
  semantic backing (`hybrid.rs`, grep-fallback) is **not implemented** — see
  `../architecture/mcp.md` gaps (Phase 2, G4). Today `search` is `ILIKE`
  substring, not semantic.
- `get_patterns` returns empty live — `file_tags` rows exist but the tagger
  that fills tag-arrays never ran (G5b); the alias plumbing itself is correct
  and unit-tested (`crates/mcp/src/lib.rs` tests around line 1267).
- `REPO_PATH` resolution (env → cwd → single-project fallback) and the
  never-silent error contract (empty vs not-indexed vs daemon-unreachable) are
  documented in `../architecture/mcp.md` § Design rationale — same code path
  for every tool above.

## Hooks — capture + the relay control channel

- Plugin manifest: `marketplace/plugins/sensei/.claude-plugin/plugin.json` —
  wires `SessionStart`, `PreCompact`, `UserPromptSubmit`, `PreToolUse`,
  `PostToolUse`, `Notification`, `Stop`, `SubagentStop`, `SessionEnd` to
  `hooks/run-hook.cmd <name>`.
- `hooks/forward` — the generic telemetry tap used by nearly every event: reads
  stdin, POSTs to daemon `/hook/event` (always 200, hooks must never block),
  falls back to `~/.sensei/events.jsonl` if the daemon is unreachable. One
  script, assistant-agnostic; a new assistant needs a new hook script only,
  zero daemon/DB changes.
- `hooks/session-start` — self-guards: injects the session-context reminder
  only if the sensei MCP server is registered (no-ops on un-init'd repos).
- `hooks/nudge` — relay-engine phone-nudge path, fired on `PreToolUse` alongside
  `forward`.
- `hooks/gate` — the relay-engine **control channel** (feature B, `docs/plan/relay-engine.md`
  §5): POSTs the `PreToolUse` payload to daemon `/hook/gate`; the daemon's JSON
  response IS Claude's `permissionDecision` (allow/deny), raised to the phone
  as a gate-card and blocking for a human answer. **Fail-open** — daemon
  down/slow/error → prints `allow`, exit 0; only an explicit human `deny`
  blocks. **Not registered in `plugin.json` by default** — opt-in via a
  `PreToolUse` matcher + `SENSEI_RELAY_GATE_TOOLS` env allowlist (activation is
  a deferred product decision, see MEMORY `project_beta_relay_plan`).
- Claude Code hook limits that shape this design: `PostToolUse` has no
  duration/token counts (→ OTLP for cost, owned by the daemon not per-session
  MCP servers, see `../architecture/mcp.md`); hooks can't call MCP tools, only
  inject text via stdout; ~100ms budget → fire-and-forget except `gate`, which
  is explicitly allowed to block up to `--max-time 55` under Claude's 60s cap.

## ACP adapters (Zed and future non-Claude agents)

- Crate: `crates/senseid/src/relay_drivers/acp.rs` — the relay P5.2
  **observe-first** ACP backend. The daemon acts as an ACP *client* watching an
  ACP-speaking agent's `session/update` stream (spec types from the
  `agent-client-protocol` SDK); `acp_update_to_segments` projects
  `SessionUpdate::Plan` entries into the same `RelaySegment` shape the Claude
  TodoWrite path produces (`dojo::relay_project`), published through the same
  `DojoClient::upsert_segments`.
- Zero-knowledge by construction: only a plan entry's `content` (short phrase)
  and mapped `SegmentState` cross the boundary; `ToolCall::raw_input` /
  `raw_output` / diff content are never read (leak-guard tests assert this).
- `drive_step` (driving the agent, not just observing) intentionally returns
  "unsupported" for ACP today — drive-over-ACP is deferred to P5.2b, along with
  the live stdio JSON-RPC connection loop itself (`agent-client-protocol` 1.x's
  client side is a builder/actor runtime, not a simple handler trait, so it
  isn't unit-testable yet; the pure mapping ships now, the loop lands in
  P5.2b). See `../plan/relay-engine.md`.
- `relay_drivers/trait_def.rs` (`RunDriver`, `DriveCapability`) is the seam:
  `ClaudeDriver` and `AcpObserveDriver` both implement it; adding a new
  ACP-speaking assistant needs no new trait, only a new driver behind the same
  `capability()`.

## Marketplace packaging

- `marketplace/` (git subtree, synced with `make marketplace-push`) packages
  four kinds: **skills** (`plugins/sensei/skills/*`, on-demand capability
  modules), **commands** (`plugins/sensei/commands/*`, phase-driving
  slash-commands), **agents** (`plugins/sensei/agents/*`, mindset subagents),
  and the **plugin** itself (registers MCP + hooks per `plugin.json` above).
- The plugin does **not** register the MCP server directly — that needs a
  repo-specific `SENSEI_REPO_PATH`, so `sensei init --mcp` writes the project's
  MCP entry; the plugin only ships hooks/skills/commands.
- Skill token budget enforced at authoring time: orientation <150 words,
  frequent <300, reference <500; frontmatter is exactly `name` + `description`.
- Static skills ship in the plugin; **generated** (stack-specific) skills are
  produced per-repo by `sensei init` — not yet covered by an automated test.
- Full rationale + diagram: [`../architecture/marketplace.md`](../architecture/marketplace.md).

## Status

| Piece | State |
|---|---|
| Claude Code + generic MCP-file registration | shipped |
| Hook capture (`forward`, `session-start`, `nudge`) | shipped, live |
| Hook gate (control channel) | shipped, fail-open, **not activated** |
| ACP observe (Zed) | shipped (pure mapping); live connection loop deferred P5.2b |
| ACP drive (control an ACP agent) | not started (P5.2b+) |
| `context_pack` / semantic `search` | not built (Phase 2, G4) |
| `get_patterns` live data | plumbing done, tagger unrun (G5b) |
</content>
