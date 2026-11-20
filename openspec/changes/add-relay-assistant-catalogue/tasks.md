# Tasks: add-relay-assistant-catalogue (#455)

Every numbered item starts with a red commit (the failing test) before the green commit (the code). Rust commits run
`cargo fmt --all --check` first. DDL is edited in `database/ddl` before any code reads it, then applied with
`dbd reconcile` (run it twice if dbd#121 bites); seed rows reach a database with `dbd import`.

## Slice 1: Seeded catalogue, installs, session host, AddMember picker

### 1. The seeded catalogue and this machine's installs
- [x] 1.1 DDL (source first): enum `sensei.tool_kind` (`assistant`, `editor`); table `sensei.assistant_catalogue`
      (`id` pk, `name`, `kind`, `aliases text[]`, `joins_crew`, `sequence`, `modified_at`); view
      `sensei.assistant_catalogue_words` (`word, id, name, kind, joins_crew`, one row per id and per alias); table
      `sensei.assistant_installs` (`catalogue_id` pk → `assistant_catalogue(id)`, `installed`, `version`, `path`,
      `config_path`, `configured`, `configured_at`, `configured_version`, `detected_at`, `modified_at`), each with
      column comments. The staging pair `database/ddl/table/staging/assistant_catalogue.ddl` +
      `database/ddl/procedure/staging/import_assistant_catalogue.ddl` (upsert on `id` guarded by `modified_at`, as
      `import_providers.ddl:3-30` does) and `database/import/staging/assistant_catalogue.jsonl` with the 13 rows in
      the design table. Delete `database/ddl/table/sensei/assistants.ddl` after asserting
      `select count(*) from sensei.assistants` is 0 on the dev and test DBs; check how the installed dbd drops a
      table whose file is gone and record it in the commit. Fix the `activity.sessions` comment that names
      `sensei.assistants.family` (`database/ddl/table/activity/sessions.ddl:90`). `dbd reconcile` then
      `dbd import` on the dev, test and e2e DBs (`make test-template` after).
- [x] 1.2 Red (pgTAP, `database/tests/`): after import, `assistant_catalogue_words` resolves `claude-code` to
      `claude` / Claude Code / assistant and `cursor` to Cursor / editor; `claude-desktop` has `joins_crew = false`;
      every `assistant_installs.catalogue_id` must exist in the catalogue (FK refuses `muse`).
- [x] 1.3 Red (unit + DB test, serial): `assistants::sync_installs(pg)` with the probe injected (two installed,
      one configured) upserts exactly those catalogue rows with `detected_at`; a second run updates in place; a
      registry id with no catalogue row is not stored. A unit test pins that every `all_assistants()` id
      (`crates/senseid/src/assistants/mod.rs:24-117`) resolves through the catalogue seed (read from the jsonl), so
      a registry entry without a seed row fails the build's tests.
- [x] 1.4 Green: `sync_installs` in `crates/senseid/src/assistants/mod.rs` (the probe stays `detect()`,
      `:178-180`); `POST /api/assistants/detect` runs it and returns the stored rows; `GET /api/assistants/installs`
      reads them joined to the catalogue (500 on a failed read, never `[]`); `assistant_configure`, `remove` and
      `upgrade` (`api/handlers/config.rs:138-177`) call it after acting; the daemon runs it once at boot when
      `assistant_installs` is empty (beside `engine_boot::spawn()`, `crates/senseid/src/main.rs:295`). Routes beside
      `api/routes.rs:464-470`.
- [x] 1.5 Red, then green (app): the setup wizard's assistants stage (`app/src/routes/(config)/stages.ts:44-45`)
      posts `/api/assistants/detect` on entry (first run); Settings → Assistants
      (`app/src/routes/(preferences)/settings/assistants/+page.ts`, `settings-reads.ts:37-48`) reads
      `/api/assistants/installs`, shows "detected <when>" and a "Detect again" action, and shows an error state on a
      failed read. Specs: `settings-reads.spec.ts` and the wizard loader spec. `assistants-configure.spec.ts` and
      `configure-assistants.spec.ts` stay green.
- [x] 1.6 Red, then green: the names the detection endpoints serve come from the catalogue (the registry's own
      `name` fields at `assistants/mod.rs:33-115` and `ClaudeCodeAssistant::name`, `assistants/claude_code.rs:435`,
      are removed); the existing `detect_families` tests stay green.

### 2. Session host and attribution
- [x] 2.1 DDL: `activity.sessions.host text` with its comment (the editor the session ran in; null for a terminal
      session or one with no evidence; never guessed). Edit the full file, then `dbd reconcile` on the dev and
      test DBs.
- [x] 2.2 Red: `TranscriptAdapter::host()` pinned per adapter (cursor → `cursor`, zed → `zed`, vscode → `vscode`;
      claude_code, copilot_cli, opencode → none). The method has no default.
- [x] 2.3 Green: add `host()` to the trait (`transcript/mod.rs:426-431`) and to every adapter.
- [x] 2.4 Red (DB test, serial): ingesting a Cursor transcript writes `sessions.host = 'cursor'` and its model,
      both for a new session and for one that already had a row with `meta_synced_at` null. A Claude Code
      transcript leaves `host` null. A metadata write with no host does not clear a set one.
- [x] 2.5 Green: `SessionMetadataPatch` (`db/pg_store/mod.rs:1759`) gains `host`. `set_session_metadata`
      (`db/pg_store/sessions.rs:1418-1454`) sets `host = COALESCE($host, host)`. The transcript coordinator's
      `set_session_metadata` (`transcript/mod.rs:710-747`) passes `adapter.host()`. Update the other
      `SessionMetadataPatch` user (`db/pg_store/tests/typed_rows_tests.rs:689`). `SessionEventRow` and
      `record_session_event` are not touched.
- [x] 2.6 Red, then green: `get_model_effectiveness` counts a Cursor-hosted session under its model, and no row
      is named `cursor` (`db/pg_store/metrics.rs:1205`). The sessions API serves `host`
      (`db/pg_store/sessions.rs:250-273`).

### 3. The catalogue read
- [x] 3.1 Red (handler test, serial DB): `GET /relay/catalogue` with seeded sessions (a `claude` session today, a
      `cursor`-hosted session with model `claude-sonnet-4-5`, a 9-day-old `codex` session, a `cowork` session)
      and seeded `assistant_installs` rows (Cursor installed + configured with a config path) returns Claude Code
      with `sessions_week: 1`, Codex with `sessions_week: 0` and its `last_used_at`, Cursor under editors with
      `models: ["claude-sonnet-4-5"]` and `mcp_config` from its install row, `cowork` under `unknown` with
      `kind: "unknown"`, and no Claude Desktop in either group. A tool with no install row has
      `installed: null`. A failed session or catalogue read is a 500, never empty lists. No filesystem probe.
- [x] 3.2 Green: the store query (joins `assistant_catalogue_words` and `assistant_installs`), the handler, and the
      route beside `/relay/now` (`api/routes.rs:133`).
- [x] 3.3 Red, then green (handler tests): every relay read that carries a lane's family also serves its
      catalogue name and kind, from one helper (`CatalogueNames`, one query per read). The Now lane
      (`now_lane_json`, `db/pg_store/relay.rs:1497-1531`) gains `family_name`, `family_kind`, `host_name` and
      `host_kind`; the Now run task `lane`/`reviewer` (`relay.rs:7213-7218`), the run detail `members`
      (`relay.rs:4885-4895`) and the task view `lane` (`relay.rs:7700-7712`) gain `family_name`. An unknown word
      gets its readable words as the name and `kind: "unknown"`. No DDL: `database/ddl/view/relay/now.ddl:15`
      keeps the raw words.

### 4. The picker
- [x] 4.1 Red: `packages/state/relay` specs. The client reads `/relay/catalogue`. `toAddMember` takes the
      catalogue and the picked id and gives the two groups (name, "last used 2h ago" or "not used yet",
      "3 sessions this week", editor models and config); `unknown` tools are not offered. Picking an assistant or
      an editor changes the note as the design says. A read failure gives an error, not empty groups.
      `assistantLabel` takes the served `family_name` at all its callers (`now-selectors.ts:120`,
      `crew-selectors.ts:228,247,254`, `run-selectors.ts:65,157`, `task-selectors.ts:133`), and `FAMILY_NAMES` is
      deleted. Update `now-selectors.spec.ts:55` (`gemini-cli` reads "Gemini CLI" only when served).
- [x] 4.2 Green: types, client, selectors, and state (`readCatalogue` when the Crew view opens, like `readRuns`
      at `app/src/lib/relay/RelayScreen.svelte:173-175`; `pickTool(id)`).
- [x] 4.3 Red, then green: `AddMember.svelte` component spec. It renders Assistants and Editors as a pressed-state
      list (`data-catalogue-group`, `data-catalogue-item=<id>`, `aria-pressed`) and the error Notice with Retry.
      Add `/preview` fixtures `AddMember/catalogue`, `AddMember/catalogue-editor-picked` and
      `AddMember/catalogue-error` to `app/src/lib/relay/catalogue.ts`, and check them in the capture matrix
      (light/dark × wide/phone × kanji/icon).
- [x] 4.4 e2e `app/e2e/tests/relay-catalogue.spec.ts` (written red before 4.2 and 4.3, green after). Seed with psql
      on `sensei_e2e` only, guarded by `/health` `dbName` as in `seed-ticket.ts`: a `claude` session now and a
      `cursor`-hosted session with model `claude-sonnet-4-5`, each with a fixed `client_session_id` and
      `on conflict do nothing`, so a re-run on a kept DB does not double the count; and an `assistant_installs`
      row for Cursor (installed, configured, the isolated home's `.cursor/mcp.json`). Call `seedSenseiMcp()`
      (`app/e2e/isolated-home.ts:112`) to write the Cursor config. Open the Crew view and assert, with no reload:
      - Assistants lists Claude Code with "1 session this week".
      - Editors lists Cursor with `claude-sonnet-4-5` and the `.cursor/mcp.json` path.
      - Picking Cursor changes the note to name Cursor's agent chat and the config.
      - Picking Claude Code names a Claude Code session.
      - The paste line text is unchanged.

      Assert only what the spec seeded; never assert that a tool is absent.

## Slice 2: Host detection at register (detected wins; self-report is the fallback)

### 5. Detection
- [ ] 5.1 Check the facts on this machine before pinning them: the `clientInfo.name` that Claude Code, Cursor and
      Zed send (log one `initialize` from each), and `TERM_PROGRAM` / `CURSOR_TRACE_ID` in a Cursor terminal, a
      Zed terminal, and Terminal.app. Record the values in design.md and correct the flowchart if one differs.
- [ ] 5.2 Red: `crates/mcp` unit tests for `detect_host(client_name, env)` over the flowchart cases, with env
      injected as a map and no process globals (Cursor client → `cursor`; Claude Code client with
      `TERM_PROGRAM=zed` → `zed`; `vscode` + `CURSOR_TRACE_ID` → `cursor`; `Apple_Terminal` → none; an unknown
      client name with no `TERM_PROGRAM` → none). The register request built by `relay_claim_request`
      (`crates/mcp/src/lib.rs:473`, keys at `:479-503`) carries `host_detected` when detection finds an editor and
      omits it otherwise, and still forwards a self-reported `host`.
- [ ] 5.3 Green: `main.rs` keeps `params.clientInfo.name` from `initialize` (`crates/mcp/src/main.rs:174-177`;
      `handle_initialize`, `lib.rs:1084-1090`, is unchanged) and reads the environment once at start; both reach
      `handle_call_tool` (`main.rs:200-205`), which adds `host_detected` to the `relay_register` args before the
      request is built. `detect_host` lives in `lib.rs` beside the request builders.
- [ ] 5.4 Red (unit): `resolve_host(reported, detected, catalogue)` in `crates/senseid/src/relay/` — detected
      catalogue editor wins over a different report and returns a mismatch; detected alone is used; report alone
      is used (fallback); a detected value the catalogue does not know as an editor (`muse`, `claude`) is ignored
      and the report stands; blank report and no detection give none; an unknown reported word is kept as given.
- [ ] 5.5 Red (handler test, serial DB): a register with `host_detected: "zed"` and no host stores host `zed`; a
      register with `host: "cursor"` and `host_detected: "zed"` stores `zed`; a register with only
      `host: "cursor"` stores `cursor`; a `host_detected` longer than 64 characters is refused, like `host`
      (`api/handlers/relay.rs:5618-5630`).
- [ ] 5.6 Green: `RegisterBody.host_detected` (`api/handlers/relay.rs:195-210`), `RelayRegisterInput`
      (`db/pg_store/relay.rs:58-60`), and the store (`relay.rs:1600-1606`) call `resolve_host` with the catalogue
      and `tracing::warn!` on a mismatch or an ignored detection, naming the lane code and both values. No DDL:
      `relay.lanes.host` keeps the resolved host.
- [ ] 5.7 e2e `app/e2e/tests/relay-host.spec.ts` (red first). Use the `RelayCrew` driver
      (`app/e2e/lib/relay-crew.ts:84`) on a clean slate, as `relay-run.spec.ts` does. Register a coordinator, a
      lane with only `host_detected: 'zed'`, a lane with `host: 'cursor', host_detected: 'zed'`, and a lane with
      only `host: 'cursor'`. Their cards appear live with no reload and read "in Zed", "in Zed" and "in Cursor".
      A terminal lane shows no tag.

## Close-out (both slices)
- [ ] 6.1 Gates green: `make test-fast`, the serial DB suite (`--test-threads=1`), `cargo clippy -- -D warnings`,
      `cargo fmt --all --check`, the app and packages vitest and lint, and `make test-app-e2e` including
      `relay-run.spec.ts`, `assistants-configure.spec.ts` and `configure-assistants.spec.ts`. The run journey's
      host tag assertion at lines 192-208 must still pass. Check for a live e2e run with `pgrep -f "instance e2e"`
      before starting one, and confirm the brew daemon is back on :7744 afterwards.
- [ ] 6.2 `openspec validate add-relay-assistant-catalogue --strict`. Update `docs/prd/modules/03-delivery/21-relay/`
      wherever it describes the crew or the catalogue.
- [ ] 6.3 Raise the follow-up issue: live hook-captured sessions report their host (add `TERM_PROGRAM` to the
      plugin hook payload). Then add launch templates to the picker (#455 deferred them). Close #455 with
      `Closes #455` on the final merge commit.
