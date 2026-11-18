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
- `sensei.assistants` exists but nothing reads or writes it (`database/ddl/table/sensei/assistants.ddl:3-16`; no
  `insert into`, `from` or `update` of it anywhere under `crates/` or `database/`). Its columns (`version`,
  `path`, `configured`, `configured_at`, `configured_version`) are per-machine install state, keyed by the
  `assistant_family` enum, which mixes harnesses and editors. Installed-tool detection is code and is never
  stored: `all_assistants()` in `crates/senseid/src/assistants/mod.rs:24-117`, probed live on every read by
  `detect()` (`:178-180`) behind `GET /api/assistants/detect` and `/api/assistants/families`
  (`api/routes.rs:464-465`, `api/handlers/config.rs:124-130`). Each entry knows its config path
  (`AssistantStatus.config_path`, `assistants/mod.rs:123-135`).
- Seeded reference data follows one convention: a `staging.<t>` table, a `staging.import_<t>()` procedure that
  upserts into the real table guarded by `modified_at`, and `database/import/staging/<t>.jsonl`
  (`database/ddl/table/staging/providers.ddl`, `database/ddl/procedure/staging/import_providers.ddl:3-30`,
  `database/import/staging/providers.jsonl`; `database/design.yaml:223-228`). New rows reach a database with
  `dbd import`.
- First run: the setup wizard's `assistants` stage (`app/src/routes/(config)/stages.ts:44-45`,
  `/setup/assistants`) loads `tryDetectAssistantFamilies` (`app/src/lib/setup/loaders.ts:109`) and configures
  through `POST /api/assistants/configure` (`config.rs:138-146`). Settings → Assistants reads the live probe
  (`app/src/routes/(preferences)/settings/assistants/+page.ts`, `settings-reads.ts:37-48`).
- The local `sensei` database is this machine's own (one daemon, one database, data in `~/.sensei/`), so a row
  in it is per-machine state.
- Model metrics group by `(provider, model)` (`crates/senseid/src/db/pg_store/metrics.rs:1205-1243`). No view or
  metric groups by `acp_id`.
- The screen names families from a fixed map (`packages/state/relay/now-selectors.ts:77-94`).
- The MCP binary answers `initialize` without reading its params: `main.rs:177` calls `handle_initialize()`
  (`crates/mcp/src/lib.rs:1084-1090`), which takes none, though `params` is in scope (`main.rs:174`).
  `handle_call_tool(params, client, cwd, base)` (`main.rs:200-205`) has no client info. The register body is
  built by the pure `relay_claim_request(tool, args)` (`lib.rs:473`), which forwards `host` among its keys
  (`lib.rs:479-503`). The daemon takes `RegisterBody.host` (`crates/senseid/src/api/handlers/relay.rs:195-202`)
  into `RelayRegisterInput.host` (`crates/senseid/src/db/pg_store/relay.rs:58-60`), trims it, and refuses one
  over `HOST_MAX_CHARS` = 64 (`relay.rs:64`, `:1600-1606`). Re-register takes only `lane_id`
  (`api/handlers/relay.rs:1103-1116`), so the host is set once, at register.
- Only the MCP process sees the client: it is the process the client starts, on stdio, with the client's
  environment. The daemon is a background service with its own environment.
- The e2e home is isolated, and `seedCursorMcpServer` writes a Cursor MCP config into it
  (`app/e2e/isolated-home.ts:101-105`). Seeding with psql into `sensei_e2e` is an established pattern
  (`app/e2e/lib/seed-ticket.ts`).

## Goals / Non-Goals
- Goals: one seeded catalogue classifies every supported tool; this machine's installs are detected once and
  kept; a session knows its editor; the person picks from what this machine has seen; the host is detected,
  not taken on the session's word.
- Non-Goals: launch templates and terminal commands (the issue defers them); renaming or splitting the
  `assistant_family` enum; host detection for live hook-captured sessions (see Decisions defaulted).

## Decisions
- **The catalogue is seeded data (user decision, 2026-11-18).** A new table `sensei.assistant_catalogue`
  lists every SUPPORTED tool: `id text primary key`, `name text not null`, `kind sensei.tool_kind not null`
  (new enum: `assistant`, `editor`), `aliases text[] not null default '{}'`, `joins_crew boolean not null
  default true`, `sequence integer`, `modified_at timestamptz`. It is seeded by the repo's import convention:
  `staging.assistant_catalogue`, `staging.import_assistant_catalogue()` (upsert on `id`, guarded by
  `modified_at`), and `database/import/staging/assistant_catalogue.jsonl`. A view
  `sensei.assistant_catalogue_words` unnests `id` and `aliases` into `(word, id, name, kind, joins_crew)` so
  every reader resolves any spelling the system writes with one join. Seed rows:

  | id | aliases | name | kind | joins_crew |
  |---|---|---|---|---|
  | `claude` | `claude-code` | Claude Code | assistant | yes |
  | `claude-desktop` | | Claude Desktop | assistant | no |
  | `codex` | | Codex | assistant | yes |
  | `copilot` | `copilot-cli`, `copilot_cli` | Copilot CLI | assistant | yes |
  | `opencode` | | OpenCode | assistant | yes |
  | `aider` | | Aider | assistant | yes |
  | `gemini` | `gemini-cli` | Gemini CLI | assistant | yes |
  | `continue` | | Continue | assistant | yes |
  | `cursor` | | Cursor | editor | yes |
  | `zed` | | Zed | editor | yes |
  | `vscode` | | VS Code | editor | yes |
  | `windsurf` | | Windsurf | editor | yes |
  | `kiro` | | Kiro | editor | yes |

  Claude Desktop is listed because setup configures it and its install row must reference the catalogue;
  `joins_crew = false` keeps it out of the AddMember picker (it does not run in a repository).
- **Unknown is unknown.** A word with no catalogue row reads `kind: "unknown"`, never an assistant or an
  editor. Its display name is its own words made readable (the way `assistantLabel` does now), which names it
  without classifying it. Supporting a new tool is a new seed row, not a code change.
- **Installed state is detected on first run and stored per machine (user decision, 2026-11-18).** A new table
  `sensei.assistant_installs` holds this machine's view: `catalogue_id text primary key references
  sensei.assistant_catalogue(id)`, `installed boolean not null`, `version text`, `path text`, `config_path text`,
  `configured boolean not null default false`, `configured_at timestamptz`, `configured_version text`,
  `detected_at timestamptz not null`, `modified_at timestamptz not null default now()`. It replaces
  `sensei.assistants`, which nothing reads or writes; that table's DDL file is deleted and the empty table
  dropped. How dbd drops a table whose file is gone (`reconcile --prune` or an explicit drop) is checked
  against the installed dbd in task 1.1, after asserting the table is empty.
  - `assistants::sync_installs(pg)` runs the existing probe (`detect()`), maps each registry id to its catalogue
    row through `assistant_catalogue_words`, and upserts one row per catalogue tool. A registry id with no
    catalogue row is logged as a warning and not stored (the seed is the list of supported tools; a test pins
    that every registry id has one).
  - It runs on first run (the wizard's assistants stage posts `POST /api/assistants/detect` on entry, and the
    daemon runs it once at startup when `assistant_installs` is empty), after every configure / remove /
    upgrade (so `configured` follows), and from Settings → Assistants "Detect again".
  - Settings → Assistants reads the stored rows joined to the catalogue (`GET /api/assistants/installs`) instead
    of probing on every load, and shows when they were detected. The wizard's family cards keep their live
    probe for the configure flow (unchanged).
  - A tool never detected has no row: readers say "not detected yet" (null), never `installed: false`.
- **One list of names (DRY).** Today names live in three places: the screen's `FAMILY_NAMES`
  (`packages/state/relay/now-selectors.ts:77-87`), the detection registry's `name` fields
  (`crates/senseid/src/assistants/mod.rs:33-115`, plus `ClaudeCodeAssistant::name` in
  `assistants/claude_code.rs:435`), and nowhere for kind. After this change the seeded catalogue owns them. The
  registry keeps its ids and probes only; the names it serves come from the catalogue through the install rows.
  `assistantLabel` is fed from four relay reads, not one, and each must carry the served name before
  `FAMILY_NAMES` can go:
  - the Now lane (`now_lane_json`, `crates/senseid/src/db/pg_store/relay.rs:1497-1531`): `family_name` and
    `host_name` (plus `family_kind` / `host_kind`);
  - the Now run task's `lane` and `reviewer` (`relay.rs:7213-7218`, built in SQL): `family_name`;
  - the run detail `members` (`relay.rs:4885-4895`): `family_name`;
  - the task view `lane` (`relay.rs:7700-7712`): `family_name`.

  One Rust helper, `CatalogueNames`, loads `assistant_catalogue_words` once per read (one query) and fills them
  (the SQL-built run task lanes are post-processed, like `now_lane_json`). A catalogue read failure fails the
  read (500), never unnamed lanes. `FAMILY_NAMES` is then deleted, and `assistantLabel` takes the served name,
  falling back to the readable words only when the daemon sends none. Two visible names change: `copilot` reads
  "Copilot CLI" (was "Copilot"); `cowork`, which the catalogue does not list, still reads "Cowork" from its
  words, with kind `unknown`.
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

  A tool counts as seen when its install row says installed or a session names it (`acp_id` for an assistant;
  `host`, or an `acp_id` that is an editor, for an editor). Only catalogue rows with `joins_crew` are served in
  `assistants` / `editors`; a seen word with no catalogue row is served in a third list, `unknown`
  (`{id, name, kind: "unknown", last_used_at}`), which the picker does not offer. `installed` comes from
  `assistant_installs` (null when never detected); the read never probes the filesystem. `sessions_week` counts sessions started in the last
  7 days. `models` lists the distinct non-null `sessions.model` of that editor's sessions, most recent first.
  `mcp_config` is the install row's `config_path` when `configured`, and null otherwise. Order:
  assistants by `sessions_week`, then `last_used_at`; editors by `last_used_at`. Nulls mean not reported (D83).
  A read failure is an error state on the screen, never an empty list.
- **The picker.** `RelayAddMemberView` gains `catalogue: { assistants, editors, picked } | null` and an error.
  Picking sets `picked`, and the note under the line names the target. For an assistant: "Paste it into a
  Claude Code session opened in this workspace's repository." For an editor: "Paste it into Cursor's agent chat.
  sensei's tools are set up in ~/.cursor/mcp.json." With nothing picked, today's note stays. The line text does
  not change.
- **The host is detected; the self-report is the fallback (user decision, 2026-11-18).** Detection lives in the
  MCP binary, the only process that sees the client. `main.rs` keeps `params.clientInfo.name` from
  `initialize` and reads the environment once at start; both are passed to `handle_call_tool`. A pure function
  `detect_host(client_name, env) -> Option<&'static str>` in `crates/mcp/src/lib.rs` gives an editor id or none,
  and `relay_register`'s body gains `host_detected` when it gives one:

  ```mermaid
  flowchart TD
    A[clientInfo.name] -->|names an editor: cursor*, zed, visual studio code, windsurf, kiro| H[that editor]
    A -->|an assistant, other, or absent| B{TERM_PROGRAM}
    B -->|zed| Z[zed]
    B -->|vscode and CURSOR_TRACE_ID set| C[cursor]
    B -->|vscode| V[vscode]
    B -->|anything else or absent| N[no detection]
  ```

  The daemon resolves the lane's host with a pure `resolve_host(reported, detected, catalogue)`:
  1. a `host_detected` that the catalogue knows as an `editor` → that is the host;
  2. else the trimmed, non-blank self-reported `host` → that is the host (a word the catalogue does not know is
     stored as given and reads `kind: "unknown"`);
  3. else no host.

  When both are set and differ, the daemon logs a warning (`relay: lane <code> reported host <r>, detected <d>;
  using <d>`) and the detected value wins. A `host_detected` that is not a catalogue editor is ignored with a
  warning, never stored. `relay.lanes.host` keeps the resolved value; no new column. A lane registered over
  plain HTTP (no MCP) sends no `host_detected`, so its self-report stands.

## Risks / Trade-offs
- The client names and env values in the flowchart come from each client's current behaviour. Task 5.1 checks
  them on this machine before the mapping is pinned. An unmatched value gives no detection, and the
  self-report stands, which is the same as today.
- Detected wins even when the session knows better (for example, a terminal opened inside an editor runs a
  terminal assistant whose `TERM_PROGRAM` names the editor). The user chose detection as the authority; the
  warning log keeps the disagreement visible.
- The detection signal table (client names, `TERM_PROGRAM` values) is code in `crates/mcp`, because the MCP
  binary has no database. It only proposes an editor id; the daemon accepts it only if the catalogue says that
  id is an editor.
- `installed` comes from the probe, which uses `/Applications` and PATH. The isolated e2e HOME does not hide
  `/Applications`, so e2e seeds `assistant_installs` rows directly and asserts only what it seeds, never what is
  absent.

## Migration Plan
New: enum `sensei.tool_kind`, tables `sensei.assistant_catalogue` and `sensei.assistant_installs`, view
`sensei.assistant_catalogue_words`, the staging pair and seed file, and the nullable `activity.sessions.host`.
Applied with `dbd reconcile` (run it twice if dbd#121 bites), then `dbd import` to load the seed (a new seed
reaches a database through import, not deploy). Removed: `sensei.assistants` (empty, never written; the task
asserts `count(*) = 0` before the drop). No backfill:
sessions ingested before this change have no host until their transcript is ingested again. Because the host
rides `set_session_metadata`, a separate reset of `meta_synced_at` for editor-sourced sessions would refill them;
it is not part of this change.

## Decided by the user (2026-11-18)
- **The supported tools are a seeded catalogue in the database** (`sensei.assistant_catalogue`, dbd import
  seed), each row `kind` = `assistant | editor`. This replaces the spec's earlier code list
  (`assistants/catalogue.rs`), which is removed from the design.
- **What is installed is detected on first run and managed in Settings**, stored per machine in
  `sensei.assistant_installs`, each row referencing the catalogue.
- **The relay reads resolve names from the seeded catalogue** (`family_name`, `host_name`).
- **Unknown tools read `unknown`**, never guessed as an assistant or an editor.
- **The host is detected**: MCP client name first, then `TERM_PROGRAM`; the lane's self-reported host only as
  the fallback. A mismatch is logged and the detected value wins. This replaces the earlier default
  ("detection is a check; the self-report wins").
- **Work and metrics attribute to the assistant (its model)**; the editor is the session's host.

## Decisions defaulted (user may override)
- **Live hook-captured sessions keep `host` null here.** The hook stream does not carry the environment. Adding
  `TERM_PROGRAM` to the plugin hook payload is a follow-up issue raised at close-out (task 6.3).
- **Launch templates are deferred**, as the issue says. The picker words the paste line and adds no `cli` field.
- **Claude Desktop is in the catalogue but not the picker** (`joins_crew = false`): setup configures it, so its
  install row needs a catalogue row, but it cannot join a crew.
- **No stored record of a host mismatch** beyond the warning log. Storing the self-report beside the resolved
  host is a follow-up if the log proves too thin.
- **First-run detection also runs at daemon startup when no install rows exist**, so a machine that skipped the
  wizard still gets its installs.
