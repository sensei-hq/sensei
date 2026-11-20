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
  (`crates/mcp/src/main.rs:177` calls `handle_initialize()` with no arguments, `crates/mcp/src/lib.rs:1084`), and
  `handle_call_tool` never sees them (`main.rs:200-205`), so nothing detects the host from the client or its
  environment. The daemon cannot do it itself: it is a background service, and only the MCP process the client
  starts sees the client's name and environment.

## What Changes
- **Classification (user decision, 2026-11-18).** The supported tools are a seeded catalogue in the database
  (`sensei.assistant_catalogue`, loaded by `dbd import` from `database/import/staging/assistant_catalogue.jsonl`):
  each row has a display name, a `kind` (`assistant | editor`) and its aliases. A tool the catalogue does not
  know reads as `unknown`: it is never guessed to be an assistant or an editor.
- **Installs (user decision, 2026-11-18).** What is installed on this machine is detected on first run (the setup
  wizard's assistants step, or daemon start with no rows) and kept in `sensei.assistant_installs`, one row per
  catalogue tool; configure/remove/upgrade keep it current and Settings → Assistants reads it and can detect
  again. It replaces the unused `sensei.assistants` table.
- **Session host.** `activity.sessions` gains `host` (the editor). Every transcript adapter declares its host:
  Cursor, Zed and VS Code transcripts set it; terminal assistants (Claude Code, Copilot CLI, OpenCode) leave it
  null. The sessions API serves it. Model metrics already group by `(provider, model)`
  (`crates/senseid/src/db/pg_store/metrics.rs:1205-1217`); this change pins that an editor-hosted session counts
  under its model and never under the editor.
- **Catalogue read.** `GET /relay/catalogue` serves what sensei has seen on this machine, in two groups.
  Assistants: name, last used, sessions this week. Editors: name, last used, models seen, and where sensei's MCP
  config was found. A seen tool the catalogue does not know is served under `unknown` and is not offered in
  the picker.
- **AddMember picker.** AddMember lists Assistants and Editors from that read. Picking one words the line for it:
  a terminal session for an assistant, or the editor's agent chat plus the config location for an editor. The
  line stays the paste line. Launch templates come later (issue: "launch templates later").
- **Host detection (user decision, 2026-11-18).** The host is detected, not taken on the session's word. The
  MCP server keeps `initialize.clientInfo.name` and, at `relay_register`, sends `host_detected`: the editor the
  client name names, else the editor `TERM_PROGRAM` (with `CURSOR_TRACE_ID`) names, else nothing. The daemon
  stores the detected host when there is one and the lane's self-reported `host` only as the fallback. When both
  are set and differ, the daemon logs a warning and the detected value wins. Detection only ever yields an editor
  the catalogue knows; anything else is no detection, never a guess.

## Impact
- Specs: `assistant-catalogue` (new), `session-attribution` (new), `relay-crew` (picker, detection).
- DDL: enum `sensei.tool_kind`; tables `sensei.assistant_catalogue`, `sensei.assistant_installs`; view
  `sensei.assistant_catalogue_words`; the staging pair and seed file; `activity.sessions.host`; removes the empty
  `sensei.assistants`. Detection adds no column: `relay.lanes.host` keeps the resolved host.
- Code: `crates/senseid/src/transcript/*` (adapter `host()`), `crates/senseid/src/assistants/` (install sync),
  `crates/senseid/src/api/handlers/relay.rs` (catalogue route, register), `crates/senseid/src/db/pg_store/`
  (sessions, relay), `crates/mcp` (`main.rs` initialize capture, `lib.rs` detection + register body), `packages/state/relay` (types, client,
  selectors, state), `app/src/lib/relay/AddMember.svelte`, `RelayScreen.svelte`, the setup wizard's assistants
  stage and Settings → Assistants.
- e2e: new `app/e2e/tests/relay-catalogue.spec.ts` and `app/e2e/tests/relay-host.spec.ts`.
- Builds on `add-relay-lane-host` (done). Does not touch `add-relay-run-grouped-lanes` or
  `update-relay-task-view-attribution`.
