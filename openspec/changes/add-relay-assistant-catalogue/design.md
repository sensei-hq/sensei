## Context
Issue #455 follows #396. The lane host is built (`openspec/changes/add-relay-lane-host`, all tasks checked). This
change builds the rest of CONTRACT §3l: classification, the session host, the AddMember picker, and host detection.

Where things stand on develop (`c25d1a930`):
- `sensei.assistant_family` mixes harnesses and editors (`database/ddl/enum/sensei/assistant_family.ddl:5-29`).
  `transcript_turns.family` and `sessions.acp_id` are plain text. `assistant_events.family` casts to the enum.
- `TranscriptAdapter` has `source()` and `family()` and nothing for the editor
  (`crates/senseid/src/transcript/mod.rs:426-431`). Synthesis passes only `adapter.family()` into the session
  (`mod.rs:693`, `mod.rs:779-826`, `crates/senseid/src/db/pg_store/sessions.rs:189-226`). Synthesis runs only for
  a session with no events yet (`mod.rs:602`, `mod.rs:788-797`). The per-session metadata write,
  `set_session_metadata` (`transcript/mod.rs:710-747` into `pg_store/sessions.rs:1418-1454`), runs on every
  ingest of a new session and once more for an existing one whose `meta_synced_at` is null
  (`session_needs_meta_backfill`, `sessions.rs:1373-1385`). It COALESCEs, so a missing value never clears one.
- `sensei.assistants` exists but nothing writes it (`database/ddl/table/sensei/assistants.ddl`; no
  `INSERT INTO assistants` in `crates/`). Installed-tool detection is code: `all_assistants()` in
  `crates/senseid/src/assistants/mod.rs:24-117`, served by `/api/assistants/detect` (`api/routes.rs:464`).
  Each entry knows its config path (`AssistantStatus.config_path`, `assistants/mod.rs:123-136`).
- Model metrics group by `(provider, model)` (`crates/senseid/src/db/pg_store/metrics.rs:1205-1243`). No view or
  metric groups by `acp_id`.
- The screen names families from a fixed map (`packages/state/relay/now-selectors.ts:77-94`).
- The MCP binary answers `initialize` without reading its params (`crates/mcp/src/main.rs:177`). `relay_register`
  forwards `host` (`crates/mcp/src/lib.rs:479-497`).
- The e2e home is isolated, and `seedCursorMcpServer` writes a Cursor MCP config into it
  (`app/e2e/isolated-home.ts:101-105`). Seeding with psql into `sensei_e2e` is an established pattern
  (`app/e2e/lib/seed-ticket.ts`).

## Goals / Non-Goals
- Goals: one classification of tools; a session knows its editor; the person picks from what this machine has
  seen; the host is checked against the environment.
- Non-Goals: launch templates and terminal commands (the issue defers them); renaming or splitting the
  `assistant_family` enum; host detection for live hook-captured sessions (see Decisions defaulted).

## Decisions
- **The catalogue is code, not a table.** A static list in `crates/senseid/src/assistants/catalogue.rs`:
  `{ id, name, kind }`, keyed by the family/host word the rest of the system writes. `sensei.assistants` stays
  unused. A table nobody writes would be a second source of truth. Entries:

  | id | name | kind |
  |---|---|---|
  | `claude` (alias `claude-code`) | Claude Code | assistant |
  | `codex` | Codex | assistant |
  | `copilot` | Copilot CLI | assistant |
  | `opencode` | OpenCode | assistant |
  | `aider` | Aider | assistant |
  | `gemini` (alias `gemini-cli`) | Gemini CLI | assistant |
  | `continue` | Continue | assistant |
  | `cursor` | Cursor | editor |
  | `zed` | Zed | editor |
  | `vscode` | VS Code | editor |
  | `windsurf` | Windsurf | editor |
  | `kiro` | Kiro | editor |

  An unknown id is an assistant named from its words, the way `assistantLabel` does now. It is never an editor:
  calling something a host without evidence would move its work off the assistant.
- **One list of names (DRY).** Today names live in three places: the screen's `FAMILY_NAMES`
  (`packages/state/relay/now-selectors.ts:77-87`), the detection registry's `name` fields
  (`crates/senseid/src/assistants/mod.rs:33-115`, plus `ClaudeCodeAssistant::name` in
  `assistants/claude_code.rs:435`), and nowhere for kind. After this change the catalogue owns them. The detection
  registry takes its names from `catalogue::name(id)`, except `claude-desktop`: it is in the registry (setup still
  configures it) but not in the catalogue (see Decisions defaulted), so it keeps its own name.
  `assistantLabel` is fed from four relay reads, not one, and each must carry the served name before
  `FAMILY_NAMES` can go:
  - the Now lane (`now_lane_json`, `crates/senseid/src/db/pg_store/relay.rs:1497-1531`): `family_name` and
    `host_name`;
  - the Now run task's `lane` and `reviewer` (`relay.rs:7213-7218`, built in SQL): `family_name`;
  - the run detail `members` (`relay.rs:4885-4895`): `family_name`;
  - the task view `lane` (`relay.rs:7700-7712`): `family_name`.

  One Rust helper fills them from the catalogue (the SQL-built run task lanes are post-processed, like
  `now_lane_json`). `FAMILY_NAMES` is then deleted, and `assistantLabel` takes the served name, falling back to
  the readable words only when the daemon sends none. Two visible names change: `copilot` reads "Copilot CLI"
  (was "Copilot"); `cowork`, which the catalogue does not list, still reads "Cowork" from its words.
- **The adapter declares its host.** `TranscriptAdapter::host() -> Option<&'static str>` has no default, so a new
  adapter must decide. Cursor gives `cursor`, Zed `zed`, VS Code `vscode`. Claude Code, Copilot CLI and OpenCode
  give `None`. The host rides the metadata write, not synthesis: `SessionMetadataPatch` gains `host`, and
  `set_session_metadata` sets `host = COALESCE($host, host)`. That path runs for synthesized and already-captured
  sessions alike, and a null never clears a set host. `SessionEventRow` and `record_session_event` do not change
  (they have about 50 callers, and synthesis skips any session that already has events). `acp_id` keeps its
  meaning (the family the adapter reports), so nothing that reads it changes.
- **Transcripts reach their host through the session.** `transcript_turns` gets no new column. A turn joins its
  session by `transcript_turns.session_id = sessions.client_session_id` and takes the host from there, so the
  host is kept in one place.
- **Attribution is by model.** Effectiveness and token metrics already group by `(provider, model)`. A test pins
  that a Cursor session with model `claude-sonnet-4-5` counts under that model, and that no model row is named
  `cursor`.
- **The catalogue read.** `GET /relay/catalogue` is a person route beside `/relay/now`. It takes an optional
  `workspace_id` (unused in this change; it is kept for launch templates).

  ```json
  {
    "assistants": [{ "id": "claude", "name": "Claude Code", "kind": "assistant",
                     "installed": true, "last_used_at": "…", "sessions_week": 12 }],
    "editors":    [{ "id": "cursor", "name": "Cursor", "kind": "editor", "installed": true,
                     "last_used_at": "…", "models": ["claude-sonnet-4-5"], "mcp_config": "~/.cursor/mcp.json" }]
  }
  ```

  A tool counts as seen when detection finds it installed or a session names it (`acp_id` for an assistant;
  `host`, or an `acp_id` that is an editor, for an editor). `sessions_week` counts sessions started in the last
  7 days. `models` lists the distinct non-null `sessions.model` of that editor's sessions, most recent first.
  `mcp_config` is the detected config path when sensei is configured there, and null otherwise. Order:
  assistants by `sessions_week`, then `last_used_at`; editors by `last_used_at`. Nulls mean not reported (D83).
  A read failure is an error state on the screen, never an empty list.
- **The picker.** `RelayAddMemberView` gains `catalogue: { assistants, editors, picked } | null` and an error.
  Picking sets `picked`, and the note under the line names the target. For an assistant: "Paste it into a
  Claude Code session opened in this workspace's repository." For an editor: "Paste it into Cursor's agent chat.
  sensei's tools are set up in ~/.cursor/mcp.json." With nothing picked, today's note stays. The line text does
  not change.
- **Detection lives in the MCP binary.** It is the process the client starts, so it sees the client's name and
  environment. `initialize` keeps `params.clientInfo.name`, and `relay_register` adds `host_detected` from a pure
  function `detect_host(client_name, env)`:

  ```mermaid
  flowchart TD
    A[clientInfo.name] -->|names an editor: cursor*, zed, visual studio code, windsurf, kiro| H[that editor]
    A -->|other or absent| B{TERM_PROGRAM}
    B -->|zed| Z[zed]
    B -->|vscode and CURSOR_TRACE_ID set| C[cursor]
    B -->|vscode| V[vscode]
    B -->|terminal or absent| N[none]
  ```

  The daemon trims both values. Reported wins. A blank report takes the detected value. `host_detected` is stored
  either way. When both are set and differ, the daemon logs a warning and the member card title adds "The
  session's environment says <Editor>."

## Risks / Trade-offs
- The client names and env values in the flowchart come from each client's current behaviour. Task 3.1 checks
  them on this machine before the mapping is pinned. An unmatched value gives no detected host, which is the
  same as today.
- `installed` uses `/Applications` and PATH. The isolated e2e HOME does not hide `/Applications`, so e2e asserts
  only what it seeds and never what is absent.

## Migration Plan
Two additive nullable columns, applied with `dbd reconcile` (run it twice if dbd#121 bites). No backfill:
sessions ingested before this change have no host until their transcript is ingested again. Because the host
rides `set_session_metadata`, a separate reset of `meta_synced_at` for editor-sourced sessions would refill them;
it is not part of this change.

## Decisions defaulted (user may override)
- **Detection is a check, not a replacement.** The self-reported host wins. Detection fills a blank host and is
  stored as `host_detected`. Recommended because the session knows where it runs, and env values can leak in
  (for example, a terminal opened from inside an editor).
- **Live hook-captured sessions keep `host` null here.** The hook stream does not carry the environment. Adding
  `TERM_PROGRAM` to the plugin hook payload is a follow-up issue raised at close-out (task 4.3).
- **Launch templates are deferred**, as the issue says. The picker words the paste line and adds no `cli` field.
- **Claude Desktop is not listed.** It does not run in a repository, so it cannot join a crew.
- **The catalogue is code.** `sensei.assistants` stays unused. Dropping it is out of scope.
