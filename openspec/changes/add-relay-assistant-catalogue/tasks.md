# Tasks: add-relay-assistant-catalogue (#455)

Every numbered item starts with a red commit (the failing test) before the green commit (the code). Rust commits run
`cargo fmt --all --check` first. DDL is edited in `database/ddl` before any code reads it, then applied with
`dbd reconcile` (run it twice if dbd#121 bites).

## Slice 1: Catalogue, session host, AddMember picker

### 1. Classification
- [ ] 1.1 Red: unit tests in `crates/senseid/src/assistants/catalogue.rs`. Each id in the design table gives its
      name and kind; the aliases `claude-code` and `gemini-cli` resolve; an unknown id (`muse`) reads as an
      assistant named "Muse", never as an editor.
- [ ] 1.2 Green: add `catalogue.rs` (`Tool { id, name, kind }`, `lookup(id)`, `name(id)`, `kind(id)`).
- [ ] 1.3 Red, then green: the detection registry takes its names from the catalogue
      (`assistants/mod.rs:33-115` and `ClaudeCodeAssistant::name`, `assistants/claude_code.rs:435`). The existing
      `detect_families` tests stay green, and a new test pins that every registry id except `claude-desktop` has a
      catalogue entry (Claude Desktop stays in the registry with its own name and out of the catalogue).

### 2. Session host and attribution
- [ ] 2.1 DDL: `activity.sessions.host text` with its comment (the editor the session ran in; null for a terminal
      session or one with no evidence; never guessed). Edit the full file, then `dbd reconcile` on the dev and
      test DBs.
- [ ] 2.2 Red: `TranscriptAdapter::host()` pinned per adapter (cursor → `cursor`, zed → `zed`, vscode → `vscode`;
      claude_code, copilot_cli, opencode → none). The method has no default.
- [ ] 2.3 Green: add `host()` to the trait (`transcript/mod.rs:426-431`) and to every adapter.
- [ ] 2.4 Red (DB test, serial): ingesting a Cursor transcript writes `sessions.host = 'cursor'` and its model,
      both for a new session and for one that already had a row with `meta_synced_at` null. A Claude Code
      transcript leaves `host` null. A metadata write with no host does not clear a set one.
- [ ] 2.5 Green: `SessionMetadataPatch` (`db/pg_store/mod.rs:1759`) gains `host`. `set_session_metadata`
      (`db/pg_store/sessions.rs:1418-1454`) sets `host = COALESCE($host, host)`. The transcript coordinator's
      `set_session_metadata` (`transcript/mod.rs:710-747`) passes `adapter.host()`. Update the other
      `SessionMetadataPatch` user (`db/pg_store/tests/typed_rows_tests.rs:689`). `SessionEventRow` and
      `record_session_event` are not touched.
- [ ] 2.6 Red, then green: `get_model_effectiveness` counts a Cursor-hosted session under its model, and no row
      is named `cursor` (`db/pg_store/metrics.rs:1205`). The sessions API serves `host`
      (`db/pg_store/sessions.rs:250-273`).

### 3. The catalogue read
- [ ] 3.1 Red (handler test, serial DB): `GET /relay/catalogue` with seeded sessions (a `claude` session today, a
      `cursor`-hosted session with model `claude-sonnet-4-5`, a 9-day-old `codex` session) returns Claude Code with
      `sessions_week: 1`, Codex with `sessions_week: 0` and its `last_used_at`, and Cursor under editors with
      `models: ["claude-sonnet-4-5"]`. Detection is injected, so the test does not depend on the machine. A
      failed session read is a 500, never empty lists.
- [ ] 3.2 Green: the store query, the handler, and the route beside `/relay/now` (`api/routes.rs:133`).
      `mcp_config` comes from detection's `config_path` when it is configured.
- [ ] 3.3 Red, then green (handler tests): every relay read that carries a lane's family also serves its
      catalogue name, from one helper. The Now lane (`now_lane_json`, `db/pg_store/relay.rs:1497-1531`) gains
      `family_name` and `host_name`; the Now run task `lane`/`reviewer` (`relay.rs:7213-7218`), the run detail
      `members` (`relay.rs:4885-4895`) and the task view `lane` (`relay.rs:7700-7712`) gain `family_name`. No DDL:
      `database/ddl/view/relay/now.ddl:15` keeps the raw words.

### 4. The picker
- [ ] 4.1 Red: `packages/state/relay` specs. The client reads `/relay/catalogue`. `toAddMember` takes the
      catalogue and the picked id and gives the two groups (name, "last used 2h ago" or "not used yet",
      "3 sessions this week", editor models and config). Picking an assistant or an editor changes the note as
      the design says. A read failure gives an error, not empty groups. `assistantLabel` takes the served
      `family_name` at all its callers (`now-selectors.ts:120`, `crew-selectors.ts:228,247,254`,
      `run-selectors.ts:65,157`, `task-selectors.ts:133`), and `FAMILY_NAMES` is deleted. Update
      `now-selectors.spec.ts:55` (`gemini-cli` reads "Gemini CLI" only when served).
- [ ] 4.2 Green: types, client, selectors, and state (`readCatalogue` when the Crew view opens, like `readRuns`
      at `app/src/lib/relay/RelayScreen.svelte:173-175`; `pickTool(id)`).
- [ ] 4.3 Red, then green: `AddMember.svelte` component spec. It renders Assistants and Editors as a pressed-state
      list (`data-catalogue-group`, `data-catalogue-item=<id>`, `aria-pressed`) and the error Notice with Retry.
      Add `/preview` fixtures `AddMember/catalogue`, `AddMember/catalogue-editor-picked` and
      `AddMember/catalogue-error` to `app/src/lib/relay/catalogue.ts`, and check them in the capture matrix
      (light/dark × wide/phone × kanji/icon).
- [ ] 4.4 e2e `app/e2e/tests/relay-catalogue.spec.ts` (written red before 4.2 and 4.3, green after). Seed with psql
      on `sensei_e2e` only, guarded by `/health` `dbName` as in `seed-ticket.ts`: a `claude` session now and a
      `cursor`-hosted session with model `claude-sonnet-4-5`, each with a fixed `client_session_id` and
      `on conflict do nothing`, so a re-run on a kept DB does not double the count. Call `seedSenseiMcp()` to write the Cursor config.
      Open the Crew view and assert, with no reload:
      - Assistants lists Claude Code with "1 session this week".
      - Editors lists Cursor with `claude-sonnet-4-5` and the `.cursor/mcp.json` path.
      - Picking Cursor changes the note to name Cursor's agent chat and the config.
      - Picking Claude Code names a Claude Code session.
      - The paste line text is unchanged.

      Assert only what the spec seeded; never assert that a tool is absent.

## Slice 2: Host detection at register

### 5. Detection
- [ ] 5.1 Check the facts on this machine before pinning them: the `clientInfo.name` that Claude Code, Cursor and
      Zed send (log one `initialize` from each), and `TERM_PROGRAM` / `CURSOR_TRACE_ID` in a Cursor terminal, a
      Zed terminal, and Terminal.app. Record the values in design.md and correct the flowchart if one differs.
- [ ] 5.2 Red: `crates/mcp` unit tests for `detect_host(client_name, env)` over the flowchart cases, with env
      injected as a map and no process globals. `relay_register` sends `host_detected` when detection finds an
      editor and omits it otherwise (`crates/mcp/src/lib.rs:479-497`).
- [ ] 5.3 Green: `initialize` keeps `params.clientInfo.name` (`crates/mcp/src/main.rs:177`). `detect_host` and the
      register body follow.
- [ ] 5.4 DDL: `relay.lanes.host_detected text` (64-char limit like `host`, `HOST_MAX_CHARS` in
      `db/pg_store/relay.rs:64`) and its comment. The `relay.now` view's lane payload adds `host_detected`
      (`database/ddl/view/relay/now.ddl:15`) and `now_lane_json` (`db/pg_store/relay.rs:1497-1531`) passes it
      through.
- [ ] 5.5 Red (handler test, serial DB): a register with `host_detected: "zed"` and no host stores host `zed`. A
      register with `host: "cursor"` and `host_detected: "zed"` keeps `cursor`, stores `host_detected = zed`,
      and logs a warning. A host_detected longer than 64 characters is refused, like `host`
      (`api/handlers/relay.rs:5618-5630`).
- [ ] 5.6 Green: `RegisterBody.host_detected`, `RelayRegisterInput`, and the store
      (`db/pg_store/relay.rs:1600-1601`).
- [ ] 5.7 Red, then green: `editorTag` in `crew-selectors.ts` adds "The session's environment says Zed." to the
      title when `host_detected` differs from `host`.
- [ ] 5.8 e2e `app/e2e/tests/relay-host.spec.ts` (red first). Use the `RelayCrew` driver
      (`app/e2e/lib/relay-crew.ts:84`) on a clean slate, as `relay-run.spec.ts` does. Register a coordinator, a
      lane with only `host_detected: 'zed'`, and a lane with `host: 'cursor', host_detected: 'zed'`. Their cards
      appear live with no reload and read "in Zed" and "in Cursor". The Cursor tag's title contains "The
      session's environment says Zed." A terminal lane shows no tag.

## Close-out (both slices)
- [ ] 6.1 Gates green: `make test-fast`, the serial DB suite (`--test-threads=1`), `cargo clippy -- -D warnings`,
      `cargo fmt --all --check`, the app and packages vitest and lint, and `make test-app-e2e` including
      `relay-run.spec.ts`. The run journey's host tag assertion at lines 192-208 must still pass. Check for a live
      e2e run with `pgrep -f "instance e2e"` before starting one, and confirm the brew daemon is back on :7744
      afterwards.
- [ ] 6.2 `openspec validate add-relay-assistant-catalogue --strict`. Update `docs/prd/modules/03-delivery/21-relay/`
      wherever it describes the crew or the catalogue.
- [ ] 6.3 Raise the follow-up issue: live hook-captured sessions report their host (add `TERM_PROGRAM` to the
      plugin hook payload). Then add launch templates to the picker (#455 deferred them). Close #455 with
      `Closes #455` on the final merge commit.
