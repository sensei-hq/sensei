# Front-door Intake — Plan 2: Intake Command + Always-the-Entry

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the user-facing front door: `/sensei:intake` runs the guided questionnaire (grounded by `intake_guide`), classifies the chunk, recommends-and-confirms a playbook, persists the run, and adopts the playbook's opening tone — plus session-start guidance and an OFF-by-default nudge hook that make intake the entry.

**Architecture:** `/sensei:intake` is a **marketplace command** (`.md` agent procedure, like `commands/analyze.md`) — the intake *conversation* is agent-driven; it orchestrates via MCP tools (`get_intake_guide` → agent classifies → `recommend_playbook` → confirm). `recommend_playbook` is enriched to return the chosen playbook's `opening_tone`. Session-start guidance names intake as the entry; a new daemon `/hook/nudge` endpoint + an OFF-by-default plugin hook nudge work started without a confirmed run.

**Tech Stack:** Marketplace plugin (`marketplace/plugins/sensei/`), Rust (`crates/senseid`, `crates/mcp`). Depends on **Plan 1** (DDL, resolver, pg_store CRUD, `recommend_playbook`). Commit per task to `develop` (approach A).

**Design:** `docs/plan/2026-07-19-frontdoor-intake-design.md`. **Prereq:** Plan 1 merged.

---

## File Structure

**Create:**
- `marketplace/plugins/sensei/commands/intake.md` — the `/sensei:intake` agent procedure.

**Modify:**
- `crates/senseid/src/db/pg_store.rs` — `session_has_confirmed_run(session_id)`; enrich the run insert path to fetch `opening_tone` (or a `get_playbook(name)` reader).
- `crates/senseid/src/api/handlers/playbook.rs` — `get_intake_guide` handler; enrich `recommend_playbook` response with `opening_tone` + `when_to_use`; `hook_nudge` handler.
- `crates/senseid/src/api/routes.rs` — `GET /api/playbook/guide`, `POST /hook/nudge`.
- `crates/mcp/src/lib.rs` — `get_intake_guide` tool schema + dispatcher branch.
- `crates/senseid/src/api/handlers/mcp_manifests.rs` — manifest for `get_intake_guide`.
- The sensei plugin's session-start guidance source (the block that lists MCP tools / workflow) — add the intake entry line. Confirm exact file during Task 4 (marketplace plugin manifest / guidance template).

---

### Task 1: `get_intake_guide` — endpoint + MCP tool

**Files:**
- Modify: `crates/senseid/src/api/handlers/playbook.rs`, `crates/senseid/src/api/routes.rs`, `crates/mcp/src/lib.rs`, `crates/senseid/src/api/handlers/mcp_manifests.rs`

- [ ] **Step 1: Write the handler** (`api/handlers/playbook.rs`) — serves the frame + per-axis rows (org-tunable at runtime, so fetched not hardcoded)
```rust
/// GET /api/playbook/guide -> { frame, axes: [{axis, prompt, help}], playbooks: [...] }
pub(crate) async fn get_intake_guide(State(state): State<AppState>) -> Json<serde_json::Value> {
    let guide = state.store.list_intake_guide().await.unwrap_or_default();
    let playbooks = state.store.list_playbooks().await.unwrap_or_default();
    let frame = guide.iter().find(|g| g["kind"] == "frame")
        .and_then(|g| g["prompt"].as_str()).unwrap_or("").to_string();
    let axes: Vec<_> = guide.into_iter().filter(|g| g["kind"] == "axis").collect();
    Json(serde_json::json!({ "frame": frame, "axes": axes, "playbooks": playbooks }))
}
```
Add `.route("/api/playbook/guide", get(handlers::playbook::get_intake_guide))` in `routes.rs`.

- [ ] **Step 2: Add the MCP tool** (`crates/mcp/src/lib.rs`) — schema (no required args) + dispatcher
```rust
tool("get_intake_guide",
    "Load the intake guide (grounding frame + per-axis elicitation prompts + the playbook \
     catalog) to run /sensei:intake. Call at the start of an intake before asking the user anything.",
    &[],
    &[]),
```
Dispatcher branch:
```rust
"get_intake_guide" => Some(DaemonRequest::get("/api/playbook/guide")),
```

- [ ] **Step 3: Add the manifest entry** for `get_intake_guide` in `mcp_manifests.rs` (Query kind).

- [ ] **Step 4: Build + verify**

Run: `cargo build -p senseid -p sensei-mcp 2>&1 | tail -3` (Expected: clean)
```bash
curl -s localhost:7744/api/playbook/guide | python3 -c "import sys,json;d=json.load(sys.stdin);print('frame' in d, len(d['axes']), len(d['playbooks']))"
```
Expected: `True 3 6`.

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/api/handlers/playbook.rs crates/senseid/src/api/routes.rs crates/mcp/src/lib.rs crates/senseid/src/api/handlers/mcp_manifests.rs
git commit -m "feat(senseid,mcp): get_intake_guide endpoint + MCP tool (frame + axes + catalog)"
```

---

### Task 2: Enrich `recommend_playbook` with the chosen playbook's opening tone

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`, `crates/senseid/src/api/handlers/playbook.rs`

- [ ] **Step 1: Write the failing test** (`pg_store.rs` DB test) — a reader for a single playbook's tone
```rust
#[tokio::test]
async fn get_playbook_tone() {
    let Some(store) = test_store().await else { return; };
    let pb = store.get_playbook("debug_flow").await.unwrap().unwrap();
    assert_eq!(pb["name"], "debug_flow");
    assert!(pb["opening_tone"].as_str().unwrap().len() > 0);
}
```

- [ ] **Step 2: Run + confirm fail**

Run: `cargo test -p senseid get_playbook_tone 2>&1 | tail -5` (Expected: FAIL — `get_playbook` not found)

- [ ] **Step 3: Implement `get_playbook`** (`pg_store.rs`, mirror `list_playbooks`)
```rust
pub async fn get_playbook(&self, name: &str) -> Result<Option<serde_json::Value>, String> {
    let row: Option<(String, String, String, String, Option<String>)> = sqlx_core::query_as::query_as(
        "SELECT name, title, when_to_use, opening_tone, method_ref
           FROM sensei.playbooks WHERE name = $1"
    ).bind(name).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(row.map(|(name,title,wtu,tone,mref)| serde_json::json!({
        "name":name,"title":title,"when_to_use":wtu,"opening_tone":tone,"method_ref":mref
    })))
}
```

- [ ] **Step 4: Enrich the `recommend_playbook` handler response** (`api/handlers/playbook.rs`) — after computing `rec`, fetch tone and include it
```rust
let pb = state.store.get_playbook(&rec.playbook).await.ok().flatten();
let opening_tone = pb.as_ref().and_then(|p| p["opening_tone"].as_str()).unwrap_or("").to_string();
let when_to_use  = pb.as_ref().and_then(|p| p["when_to_use"].as_str()).unwrap_or("").to_string();
// ... include in the returned json:
Json(serde_json::json!({
    "playbook": rec.playbook, "rationale": rec.rationale, "rule": rec.rule_name,
    "defaulted": rec.defaulted, "opening_tone": opening_tone, "when_to_use": when_to_use
}))
```

- [ ] **Step 5: Run tests + build**

Run: `cargo test -p senseid get_playbook_tone 2>&1 | tail -5` (Expected: PASS/skip)
Run: `cargo build -p senseid 2>&1 | tail -3` (Expected: clean)
```bash
curl -s localhost:7744/api/playbook/recommend -H 'content-type: application/json' -d '{"lifecycle":"stable","intent":"bug","risk":"low"}' | python3 -c "import sys,json;print(json.load(sys.stdin)['opening_tone'][:30])"
```
Expected: the debug_flow opening tone prefix.

- [ ] **Step 6: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs crates/senseid/src/api/handlers/playbook.rs
git commit -m "feat(senseid): recommend_playbook returns the chosen playbook's opening_tone + when_to_use"
```

---

### Task 3: `/sensei:intake` command procedure

**Files:**
- Create: `marketplace/plugins/sensei/commands/intake.md`

- [ ] **Step 1: Write the command** (mirror the frontmatter + numbered Procedure of `commands/analyze.md`)
```markdown
---
description: The front door — clarify intent, classify the chunk, and recommend-and-confirm a playbook before any work.
argument-hint: What you want to do (or omit to describe it in the dialogue)
---

## Procedure

1. Call `get_intake_guide` — MANDATORY. It returns the grounding `frame`, the per-axis
   elicitation prompts (`axes`), and the playbook `catalog`. Adopt the `frame` as your posture.
2. Run a short clarifying dialogue with the user, guided by the axis prompts — determine:
   - **lifecycle**: `greenfield` | `stable` (infer from the project's spine/existing code; ask only if unclear)
   - **intent**: `explore` | `ux` | `feature` | `enhancement` | `bug`
   - **risk**: `low` | `high` (use the code graph — get_callers/get_communities on the touched area — to judge blast-radius)
   Ask only what you cannot infer. One question at a time.
3. Call `recommend_playbook(lifecycle, intent, risk, session_id=<current session>)`. It returns
   `playbook`, `rationale`, and `opening_tone`.
4. **Recommend-and-confirm**: tell the user the recommended playbook and the one-line `rationale`.
   If `risk = high`, you MUST get explicit confirmation. On agreement, call
   `recommend_playbook(... , confirm="true")` to record the confirmed run.
5. Adopt the returned `opening_tone` as the posture for the next stage, and proceed under the
   chosen playbook. (Playbooks are named routes today; follow the tone + when-to-use.)
6. Call `log_event(type="command_invoked", data="{\"command\":\"intake\",\"args\":\"$ARGUMENTS\"}")`.
```

- [ ] **Step 2: Verify it lists + resolves** (marketplace commands are file-presence based; validate the frontmatter parses and the tool names match Plan 1/Task 1 tools)

Run:
```bash
head -5 marketplace/plugins/sensei/commands/intake.md
grep -oE 'get_intake_guide|recommend_playbook|log_event' marketplace/plugins/sensei/commands/intake.md | sort -u
```
Expected: frontmatter present; all three tool names appear (and exist as MCP tools).

- [ ] **Step 3: Commit**
```bash
git add marketplace/plugins/sensei/commands/intake.md
git commit -m "feat(marketplace): /sensei:intake front-door command (guide -> classify -> recommend-and-confirm)"
```

---

### Task 4: Session-start guidance — name intake as the entry

**Files:**
- Modify: the sensei plugin's session-start guidance source (the block that renders the `<sensei-session>` workflow/MCP guidance). Locate it first: `grep -rn "Workflow Commands\|/sensei:idea\|Phase:" marketplace/plugins/sensei/` — it lives in the plugin's session-start template/hook, not the daemon.

- [ ] **Step 1: Find the guidance block**

Run: `grep -rln "Workflow Commands\|Phase:\|/sensei:idea" marketplace/plugins/sensei/`
Expected: the file that emits the session-start guidance (a hook script or template).

- [ ] **Step 2: Add the intake entry line** — in the Workflow/entry section, add intake as the *first* step:
```
**Front door (start here):** /sensei:intake — clarify intent → recommend-and-confirm a playbook before work. Run it at the start of any new chunk.
```
Keep it a single, unmissable line ahead of the phase commands (push-not-pull framing).

- [ ] **Step 3: Verify**

Run: `grep -n "sensei:intake" marketplace/plugins/sensei/<guidance-file>`
Expected: the new line present, ahead of the phase list.

- [ ] **Step 4: Commit**
```bash
git add marketplace/plugins/sensei/<guidance-file>
git commit -m "feat(marketplace): name /sensei:intake as the session-start front door"
```

---

### Task 5: Nudge hook — `/hook/nudge` endpoint + plugin hook (OFF by default)

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`, `crates/senseid/src/api/handlers/sessions.rs` (or `playbook.rs`), `crates/senseid/src/api/routes.rs`
- Create/modify: the plugin hook config (OFF by default — present, not enabled), mirroring the relay B hook-gate posture.

- [ ] **Step 1: Write the failing test** (`pg_store.rs`) — the "no confirmed run yet" check that drives the nudge
```rust
#[tokio::test]
async fn session_confirmed_run_gate() {
    let Some(store) = test_store().await else { return; };
    let sid = store.create_session(&test_folder_id().await, "intake test", None).await.unwrap();
    assert!(!store.session_has_confirmed_run(&sid).await.unwrap());
    store.insert_playbook_run(Some(sid), None, "stable","bug","low", None, "debug_flow","r", true).await.unwrap();
    assert!(store.session_has_confirmed_run(&sid).await.unwrap());
}
```

- [ ] **Step 2: Run + confirm fail**

Run: `cargo test -p senseid session_confirmed_run_gate 2>&1 | tail -5` (Expected: FAIL — method not found)

- [ ] **Step 3: Implement `session_has_confirmed_run`** (`pg_store.rs`)
```rust
pub async fn session_has_confirmed_run(&self, session_id: &uuid::Uuid) -> Result<bool, String> {
    let row: (bool,) = sqlx_core::query_as::query_as(
        "SELECT exists(select 1 from sensei.playbook_run where session_id = $1 and confirmed)"
    ).bind(session_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(row.0)
}
```

- [ ] **Step 4: Write the `hook_nudge` handler** (mirror `hook_gate` in `sessions.rs`; FAIL-OPEN = never block)
```rust
/// POST /hook/nudge  { session_id }  ->  { nudge: bool, message?: string }
/// Non-blocking: suggests /sensei:intake when a session has started work without a confirmed run.
pub(crate) async fn hook_nudge(
    State(state): State<AppState>, Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let Some(sid) = payload["session_id"].as_str().and_then(|s| s.parse().ok()) else {
        return Json(serde_json::json!({ "nudge": false }));  // fail-open
    };
    match state.store.session_has_confirmed_run(&sid).await {
        Ok(true) => Json(serde_json::json!({ "nudge": false })),
        Ok(false) => Json(serde_json::json!({ "nudge": true,
            "message": "No playbook chosen for this chunk yet — consider /sensei:intake to pick one." })),
        Err(e) => { tracing::warn!(error=%e, "hook_nudge: db error — fail-open"); Json(serde_json::json!({ "nudge": false })) }
    }
}
```
Add `.route("/hook/nudge", post(handlers::sessions::hook_nudge))` in `routes.rs`.

- [ ] **Step 5: Add the plugin hook config OFF by default** — mirror the relay B hook-gate: the hook exists in the plugin but is NOT registered/enabled (activation is a separate Jerry-gated decision). Document in the plugin config that enabling it turns on the PreToolUse nudge that calls `/hook/nudge`.

- [ ] **Step 6: Run tests + build**

Run: `cargo test -p senseid session_confirmed_run_gate 2>&1 | tail -5` (Expected: PASS/skip)
Run: `cargo build -p senseid 2>&1 | tail -3` (Expected: clean)
```bash
curl -s localhost:7744/hook/nudge -H 'content-type: application/json' -d '{"session_id":"<a session with no confirmed run>"}'
```
Expected: `{"nudge":true,"message":"..."}`.

- [ ] **Step 7: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs crates/senseid/src/api/handlers/sessions.rs crates/senseid/src/api/routes.rs marketplace/plugins/sensei/
git commit -m "feat(senseid,marketplace): intake nudge hook — /hook/nudge endpoint + OFF-by-default plugin hook"
```

---

## Final verification (whole plan)

- [ ] `cargo test -p senseid 2>&1 | tail -20` — green.
- [ ] `cargo clippy -p senseid -p sensei-mcp 2>&1 | tail -5` — no warnings.
- [ ] `GET /api/playbook/guide` → frame + 3 axes + 6 playbooks; `recommend_playbook` → includes `opening_tone`; `/hook/nudge` → nudges only when no confirmed run.
- [ ] `/sensei:intake` command file present with valid frontmatter; every tool it names exists.
- [ ] Session-start guidance names `/sensei:intake` ahead of the phase commands.
- [ ] Nudge hook present but OFF by default (activation deferred to Jerry).
- [ ] Whole-feature code review (subagent) across Plan 1 + Plan 2 against the design.

## Self-review notes (author)

- **Spec coverage:** `/sensei:intake` (guide → classify → recommend-and-confirm → persist → tone) ✓ T1–T3; always-the-entry guidance ✓ T4; nudge hook OFF ✓ T5. Data/resolver/recommender = Plan 1.
- **Agent-driven dialogue:** the intake conversation is the agent's job (an LLM in-session), so the "command" is a marketplace `.md` procedure calling MCP tools — not a CLI binary subcommand (a binary can't hold a clarifying dialogue). `classify_chunk` (Plan 1) covers the non-agent/app-form path.
- **Type/tool consistency:** tool names (`get_intake_guide`, `recommend_playbook`, `log_event`) and axis labels match Plan 1 + the command file. `recommend_playbook` args (`lifecycle/intent/risk/session_id/feature/confirm`) match Plan 1 Task 8.
- **Deferred:** hook *activation*; the Sensei-app schema-form renderer of the guide; §9 outcome population; Dōjō org/learned authoring.
