# Change: Assistants vs editors — the catalogue, the session host, the AddMember picker, host detection (#455)

## Why
An editor runs an assistant behind the scenes, so work belongs to the assistant (its model) and the editor is
only the host. Today one word does both jobs. `assistant_family` holds harnesses and editors alike
(`database/ddl/enum/sensei/assistant_family.ddl:5-29`: `claude`, `cursor`, `zed`, `vscode`, …), every transcript
adapter reports an editor as its `family` (`crates/senseid/src/transcript/cursor.rs:248-250`,
`zed.rs:72`, `vscode.rs:706-708`), and `activity.sessions` has no column for the editor
(`database/ddl/table/activity/sessions.ddl:3-40`). Nothing classifies a tool as an assistant or an editor.

The lane `host` shipped in `add-relay-lane-host` (#396): register takes a self-reported `host`
(`crates/senseid/src/api/handlers/relay.rs:202`), `relay.lanes.host` keeps it
(`database/ddl/table/relay/lanes.ddl:17`), and the member card shows "in Cursor"
(`packages/state/relay/crew-selectors.ts:254`). Three things remain (CONTRACT §3l, gap 20; brief
`coord/briefs/relay-crew-daemon.md`):
- AddMember shows only a paste line; its picker waits for a catalogue the daemon does not serve
  (`app/src/lib/relay/AddMember.svelte:4-5`, `packages/state/relay/crew-selectors.ts:2-4`).
- Sessions and transcripts do not carry their host editor.
- The host is only what the session says. The MCP server ignores the client's `initialize` params
  (`crates/mcp/src/main.rs:177`), so nothing checks or fills it from the environment.

## What Changes
- **Classification.** A tool catalogue in senseid gives every known tool a `kind` (`assistant | editor`) and a
  display name. A tool the catalogue does not know reads as an assistant, never as an editor.
- **Session host.** `activity.sessions` gains `host` (the editor). Every transcript adapter declares its host:
  Cursor, Zed and VS Code transcripts set it; terminal assistants (Claude Code, Copilot CLI, OpenCode) leave it
  null. The sessions API serves it. Model metrics already group by `(provider, model)`
  (`crates/senseid/src/db/pg_store/metrics.rs:1205-1217`); this change pins that an editor-hosted session counts
  under its model and never under the editor.
- **Catalogue read.** `GET /relay/catalogue` serves what sensei has seen on this machine, in two groups.
  Assistants: name, last used, sessions this week. Editors: name, last used, models seen, and where sensei's MCP
  config was found.
- **AddMember picker.** AddMember lists Assistants and Editors from that read. Picking one words the line for it:
  a terminal session for an assistant, or the editor's agent chat plus the config location for an editor. The
  line stays the paste line. Launch templates come later (issue: "launch templates later").
- **Host detection.** At register, the MCP server sends `host_detected`, worked out from the MCP client name
  (`initialize.clientInfo.name`) and the environment (`TERM_PROGRAM`, `CURSOR_TRACE_ID`). The daemon keeps the
  self-reported host when there is one, fills a blank one from detection, and stores `host_detected` on the lane
  as the check. The member card title says when the two disagree.

## Impact
- Specs: `assistant-catalogue` (new), `session-attribution` (new), `relay-crew` (picker, detection).
- DDL: `activity.sessions.host`, `relay.lanes.host_detected`, the `relay.now` lane shape (dbd full files).
- Code: `crates/senseid/src/transcript/*` (adapter `host()`), `crates/senseid/src/assistants/` (catalogue),
  `crates/senseid/src/api/handlers/relay.rs` (catalogue route, register), `crates/senseid/src/db/pg_store/`
  (sessions, relay), `crates/mcp` (initialize capture, detection), `packages/state/relay` (types, client,
  selectors, state), `app/src/lib/relay/AddMember.svelte`, `RelayScreen.svelte`.
- e2e: new `app/e2e/tests/relay-catalogue.spec.ts` and `app/e2e/tests/relay-host.spec.ts`.
- Builds on `add-relay-lane-host` (done). Does not touch `add-relay-run-grouped-lanes` or
  `update-relay-task-view-attribution`.
