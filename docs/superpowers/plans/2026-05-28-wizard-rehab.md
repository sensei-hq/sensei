# Setup Wizard Rehab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the setup wizard to a working end-to-end first-run flow — fix the scan pipeline event contracts, drop the Assignments stub, repair the Done-stage routing, surface UI affordances correctly, and restore the setup-gate that was temporarily disabled during smoke testing.

**Architecture:** All changes are bug-fix-scoped. Daemon work is one new helper module that listens to the existing `TaskEvent` broadcast channel and translates per-file completions into throttled `StateEvent::folder_update` SSE events; one helper that fires `StateEvent::project_update::active` when all folders for a project reach `indexed`. App work is targeted edits to `(config)/+layout.svelte`, the Switch component, the Libraries refresh logic, the Done page commit error path, and removing the Assignments stage entirely.

**Tech Stack:** Rust (tokio broadcast, atomic counters, mutex), SvelteKit 5 (`$state`/`$derived`/`$effect`), Tauri 2, vitest, Playwright, existing `sensei`/`sensei_dev` Postgres.

**Reference spec:** `docs/superpowers/specs/2026-05-27-wizard-rehab-design.md`

---

## File map

### Daemon (Rust)

| Path | Action | Owns |
|---|---|---|
| `crates/senseid/src/tasks/progress_emitter.rs` | Create | Throttled per-folder StateEvent::folder_update emitter (subscribes to TaskEvent broadcast) |
| `crates/senseid/src/tasks/mod.rs` | Modify | Add `pub mod progress_emitter;` |
| `crates/senseid/src/tasks/executor.rs` | Modify | Start the progress emitter task at daemon boot |
| `crates/senseid/src/tasks/handlers/process.rs` | Modify | Emit `StateEvent::project_update(Active)` when all folders indexed |
| `crates/senseid/src/db/pg_store.rs` | Modify | Add `count_unindexed_folders(project_id)` helper |
| `crates/senseid/tests/scan_lifecycle.rs` | Create | End-to-end Rust integration test |

### App (TypeScript / Svelte 5)

| Path | Action | Owns |
|---|---|---|
| `app/src/lib/components/Switch.svelte` | Modify | Replace undefined `--color-primary-600` with `--color-primary-z5` |
| `app/src/routes/(config)/setup/assistants/+page.svelte` | Modify | Bump `.card-selected` border to 2px + accent color |
| `app/src/routes/(config)/+layout.svelte` | Modify | Drop disabled rail attribute; add commitError state + inline error display; reload appState after Done commit |
| `app/src/routes/(config)/stages.ts` | Modify | Remove Assignments entry |
| `app/src/routes/(config)/setup/assignments/` | Delete | Stub directory removed |
| `app/src/lib/wizard-state.svelte.ts` | Modify | Remove Assignments commit handler; `commitStage` on `done` throws on failure |
| `app/src/lib/setup/loaders.ts` | Modify | Drop Assignments from completion key list |
| `app/src/lib/setup/mock-contracts.ts` | Modify | Drop Assignments from mock completion |
| `app/src/lib/setup/contracts.spec.svelte.ts` | Modify | Drop Assignments from test fixtures |
| `app/src/lib/wizard-state.spec.svelte.ts` | Modify | Drop Assignments from completion fixtures + adjust stage-count assertions |
| `app/src/lib/setup/loaders.spec.svelte.ts` | Modify | Drop Assignments from key list assertion |
| `app/src/lib/scan-state.svelte.ts` | Modify | Drop substring filters; move `processed` to a project-derived getter |
| `app/src/routes/(config)/setup/scan/+page.svelte` | Modify | Use the new processed derivation (from `projects.items`, not `activities`) |
| `app/src/lib/scan-state.spec.svelte.ts` | Modify | Update test fixtures + `processed` test target |
| `app/src/lib/wizard-state.svelte.ts` | Modify (again) | `refreshLibraries` filter to currently-scanned repos |
| `app/src/hooks.ts` | Modify | Restore the setup gate (revert the TEMP comment-out) |
| `app/src/hooks.spec.svelte.ts` | Modify | Un-skip the gate redirect test |
| `app/e2e/tests/setup-wizard-happy-path.spec.ts` | Create | End-to-end Playwright walk through all 10 stages |

---

## Tasks

### Task 1: Drop the Assignments stage

**Files:**
- Modify: `app/src/routes/(config)/stages.ts`
- Delete: `app/src/routes/(config)/setup/assignments/+page.svelte`
- Modify: `app/src/lib/wizard-state.svelte.ts:221`
- Modify: `app/src/lib/setup/loaders.ts:13`
- Modify: `app/src/lib/setup/mock-contracts.ts:100`
- Modify: `app/src/lib/setup/contracts.spec.svelte.ts:68`
- Modify: `app/src/lib/wizard-state.spec.svelte.ts` (3 references)
- Modify: `app/src/lib/setup/loaders.spec.svelte.ts:37`

- [ ] **Step 1: Remove the Assignments stage from STAGES**

In `app/src/routes/(config)/stages.ts`, delete lines 134-145 (the `{ id: 'assignments', ... },` block — the entire object literal between Inference and Done).

- [ ] **Step 2: Remove the Assignments commit handler**

In `app/src/lib/wizard-state.svelte.ts`, delete line 221: `assignments: async () => {},`.

- [ ] **Step 3: Remove Assignments from the completion key list**

In `app/src/lib/setup/loaders.ts`, line 13: change

```typescript
'projects', 'libraries', 'instruments', 'inference', 'assignments', 'done',
```

to

```typescript
'projects', 'libraries', 'instruments', 'inference', 'done',
```

- [ ] **Step 4: Strip Assignments from mock + test fixtures**

In each of `app/src/lib/setup/mock-contracts.ts`, `app/src/lib/setup/contracts.spec.svelte.ts`, `app/src/lib/wizard-state.spec.svelte.ts`, `app/src/lib/setup/loaders.spec.svelte.ts`: remove the `assignments: 'pending'` / `assignments: 'done'` literal (always inside an object literal with the same shape as the other stage keys). Run `grep -n "assignments" app/src/` after to confirm nothing remains.

- [ ] **Step 5: Delete the assignments route directory**

```bash
rm -rf app/src/routes/\(config\)/setup/assignments
```

- [ ] **Step 6: Verify**

```bash
cd app && bunx svelte-check 2>&1 | tail -3
cd app && bunx vitest run 2>&1 | tail -5
```

Expected: 0 errors. Tests pass. (One test in `wizard-state.spec.svelte.ts` may assert stage count = 11 — change to 10. Search for `11` and `10` in that file and update.)

- [ ] **Step 7: Commit**

```bash
git add app/src/routes/'(config)'/stages.ts \
        app/src/lib/wizard-state.svelte.ts \
        app/src/lib/setup/loaders.ts \
        app/src/lib/setup/mock-contracts.ts \
        app/src/lib/setup/contracts.spec.svelte.ts \
        app/src/lib/wizard-state.spec.svelte.ts \
        app/src/lib/setup/loaders.spec.svelte.ts
git rm -r app/src/routes/'(config)'/setup/assignments
git commit -m "feat(app): drop Assignments stage from setup wizard"
```

---

### Task 2: Switch + card-selected token fix

**Files:**
- Modify: `app/src/lib/components/Switch.svelte:34`
- Modify: `app/src/routes/(config)/setup/assistants/+page.svelte:84-90`

- [ ] **Step 1: Fix the Switch "on" background token**

In `app/src/lib/components/Switch.svelte`, change line 34:

```css
.switch.on {
    background: oklch(var(--color-primary-z5) / 1);
}
```

(Was `--color-primary-600` which is not defined anywhere in the project's design tokens.)

- [ ] **Step 2: Strengthen the card-selected affordance**

In `app/src/routes/(config)/setup/assistants/+page.svelte`, replace the `.card-selected` rule with:

```css
.card-selected {
    border: 2px solid oklch(var(--color-primary-z5) / 1);
    background: oklch(var(--color-surface-z2) / 1);
}
.card-selected .chip {
    background: oklch(var(--color-surface-z1) / 1);
}
```

- [ ] **Step 3: Verify**

```bash
cd app && bunx svelte-check 2>&1 | tail -3
```

Expected: 0 errors.

Visual smoke (optional — for confidence, not gating): start `make app-dev-bundle`, navigate to `/setup/assistants` after toggling `setup_complete='0'`, click a Switch, observe both card border and switch thumb-track change color.

- [ ] **Step 4: Commit**

```bash
git add app/src/lib/components/Switch.svelte \
        app/src/routes/'(config)'/setup/assistants/+page.svelte
git commit -m "fix(app): Switch on-state token + assistant card-selected affordance"
```

---

### Task 3: Drop rail disabled state + investigate Instruments rendering lag

**Files:**
- Modify: `app/src/routes/(config)/+layout.svelte`

This task has an *investigation* step. The reported symptom is: rail-click on Instruments updates the header but not the body content; clicking Continue advances past Instruments to Inference; clicking Instruments again in the rail finally shows Instruments content but the active marker is still on Inference. Drop the disabled-state in the same patch.

- [ ] **Step 1: Drop the disabled attribute from rail buttons**

In `app/src/routes/(config)/+layout.svelte` find this block (around line 87-110):

```svelte
<button
    data-rail-item
    data-stage-id={s.id}
    data-active={s.active}
    class="..."
    class:active={s.active}
    class:done={isDone}
    onclick={() => {
        if (isNavigable) goto(s.path);
    }}
    disabled={!isNavigable}
>
```

Replace with:

```svelte
<button
    data-rail-item
    data-stage-id={s.id}
    data-active={s.active}
    class="..."
    class:active={s.active}
    class:done={isDone}
    onclick={() => goto(s.path)}
>
```

Remove the `{@const isNavigable = isDone || s.active}` line above the button — no longer referenced.

- [ ] **Step 2: Reproduce the rendering lag**

```bash
make install-dev
senseid-dev restart
make app-dev-bundle
```

In the running app, walk to `/setup/libraries` then click `Instruments` in the rail. Observe whether the body content updates to the Instruments page or stays on Libraries. Open the webview devtools (right-click → Inspect, or Cmd+Option+I) and check the console for any errors.

- [ ] **Step 3: Diagnose**

Three hypotheses to check, in order:

**H1 — `setActive` mutating in place doesn't trigger reactivity:**

Look at `app/src/lib/wizard-state.svelte.ts:setActive` (around line 276). The current implementation:

```typescript
setActive(id: string): void {
    for (const s of this.stages) s.active = s.id === id;
}
```

Svelte 5 `$state` arrays react to deep mutation of items if items themselves are `$state`-wrapped. But `cloneStages()` may return plain objects, in which case mutation does not re-trigger derivations. To test: in the running app, observe whether `wizardState.stages.find(s => s.id === 'instruments').active` becomes `true` in the devtools after the click. If yes → reactivity is OK, move to H2. If no → fix setActive to rebuild the array:

```typescript
setActive(id: string): void {
    this.stages = this.stages.map(s => ({ ...s, active: s.id === id }));
}
```

**H2 — Per-page `$state` retained across navigation:**

If H1 was fine, suspect per-page state. Look at the Libraries and Instruments `+page.svelte` files for component-level `$state` declarations. If both pages share the same SvelteKit layout, Svelte may keep the previous page mounted briefly during navigation. Check for any `onMount`/`onDestroy` patterns that might be missing or asymmetric.

**H3 — Order of execution between URL change and `$effect` firing:**

The layout has `$effect(() => { if (stage) wizardState.setActive(stage.id); })`. The effect fires when `stage` changes (which derives from `page.url.pathname`). If a child `+page.svelte` reads `wizardState.activeStage` synchronously during mount, it sees stale active before the effect runs. Check for `wizardState.activeStage` or `wizardState.stages.find(s => s.active)` reads in the stage page scripts.

- [ ] **Step 4: Apply the fix**

Based on the diagnosis from Step 3, apply the minimal fix. The most likely culprit is H1 (mutation-in-place); if that's it, the one-line setActive change is the entire fix.

- [ ] **Step 5: Verify the fix**

Re-build (`make app-dev-bundle`), repeat the Libraries → Instruments → Inference → Instruments navigation. Confirm:
- Body content matches the rail's active item.
- The active marker on the rail follows back-navigation (clicking Instruments after being on Inference moves the marker to Instruments).

- [ ] **Step 6: Verify tests still pass**

```bash
cd app && bunx svelte-check 2>&1 | tail -3
cd app && bunx vitest run 2>&1 | tail -5
```

Expected: 0 errors, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add app/src/routes/'(config)'/+layout.svelte app/src/lib/wizard-state.svelte.ts
git commit -m "fix(app): drop rail disabled-until-done; fix Instruments navigation lag"
```

If only the layout file changed (no setActive fix needed), drop wizard-state.svelte.ts from the `git add`.

---

### Task 4: Scan progress emitter (Rust)

**Files:**
- Create: `crates/senseid/src/tasks/progress_emitter.rs`
- Modify: `crates/senseid/src/tasks/mod.rs`

The emitter is a single async task that lives for the daemon's lifetime. It subscribes to the existing `TaskEvent` broadcast channel and:

- On `TaskEvent::FolderQueued { folder_path, files_total }` — opens an in-memory tracker for that folder.
- On `TaskEvent::Completed { kind: "process_file", folder_path }` — increments the tracker's count; if 300ms has passed OR 25 files have accumulated since the last emit, sends `StateEvent::folder_update` with current `files_completed` and `status=Indexing`.
- On `TaskEvent::Completed { kind: "build_connections", folder_path }` — emits a terminal `StateEvent::folder_update` with `status=Indexed`, then drops the tracker.

The emitter looks up the folder's UUID + project from `pg_store` on first observation. Cache lookups in the tracker so we don't query per file event.

- [ ] **Step 1: Write the throttle unit tests**

Create `crates/senseid/src/tasks/progress_emitter.rs` with `#[cfg(test)] mod tests` at the bottom (define the module first, then the tests):

```rust
//! Scan-pipeline progress emitter — translates per-file TaskEvent::Completed
//! events into throttled StateEvent::folder_update SSE events.

use crate::api::events::{StateEvent, ScanFolder, FolderKind, FolderStatus};
use crate::tasks::progress::TaskEvent;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

const THROTTLE_DURATION: Duration = Duration::from_millis(300);
const THROTTLE_FILE_DELTA: u32 = 25;

#[derive(Clone)]
struct FolderTracker {
    folder_id:       String,
    project_id:      String,
    folder_name:     String,
    path:            String,
    kind:            FolderKind,
    stack:           Vec<String>,
    files_total:     u32,
    files_completed: u32,
    last_emit_at:    Instant,
    last_emit_count: u32,
}

impl FolderTracker {
    fn should_emit(&self, now: Instant) -> bool {
        now.duration_since(self.last_emit_at) >= THROTTLE_DURATION
            || (self.files_completed - self.last_emit_count) >= THROTTLE_FILE_DELTA
    }
}

/// Test-only handle to drive the throttle without spawning the real task.
#[cfg(test)]
pub(crate) struct ThrottleDriver {
    pub tracker: FolderTracker,
}

#[cfg(test)]
impl ThrottleDriver {
    pub fn new(files_total: u32, now: Instant) -> Self {
        Self {
            tracker: FolderTracker {
                folder_id: "f".into(), project_id: "p".into(),
                folder_name: "n".into(), path: "/p".into(),
                kind: FolderKind::Git, stack: vec![],
                files_total, files_completed: 0,
                last_emit_at: now, last_emit_count: 0,
            },
        }
    }
    pub fn tick(&mut self, now: Instant) -> bool {
        self.tracker.files_completed += 1;
        if self.tracker.should_emit(now) {
            self.tracker.last_emit_at = now;
            self.tracker.last_emit_count = self.tracker.files_completed;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_after_25_files_within_throttle_window() {
        let t0 = Instant::now();
        let mut d = ThrottleDriver::new(100, t0);
        // First 24 should not emit (under both thresholds)
        for _ in 0..24 { assert!(!d.tick(t0)); }
        // 25th should emit
        assert!(d.tick(t0));
    }

    #[test]
    fn emits_after_300ms_elapsed_even_with_few_files() {
        let t0 = Instant::now();
        let mut d = ThrottleDriver::new(100, t0);
        assert!(!d.tick(t0));   // 1 file, 0ms
        // Advance 300ms
        let t1 = t0 + Duration::from_millis(300);
        assert!(d.tick(t1));    // 2 files, 300ms — should emit
    }

    #[test]
    fn resets_throttle_window_after_emit() {
        let t0 = Instant::now();
        let mut d = ThrottleDriver::new(100, t0);
        for _ in 0..25 { d.tick(t0); }      // emits at 25
        // Now files at 25, last_emit_count at 25.
        for _ in 0..24 { assert!(!d.tick(t0)); } // 49 files; delta is 24
        assert!(d.tick(t0));                     // 50 files; delta is 25 — emits
    }
}
```

- [ ] **Step 2: Run the tests — verify they fail**

```bash
cargo test --features dev -p senseid progress_emitter 2>&1 | tail -10
```

Expected: compile error — `ScanFolder` / `FolderKind` / `FolderStatus` / `StateEvent` paths need adjustment OR the file compiles but no tests defined yet.

If imports are wrong, fix them based on the actual paths in `crates/senseid/src/api/events.rs`.

- [ ] **Step 3: Add module declaration**

In `crates/senseid/src/tasks/mod.rs`, add:

```rust
pub mod progress_emitter;
```

- [ ] **Step 4: Re-run tests — verify they pass**

```bash
cargo test --features dev -p senseid progress_emitter 2>&1 | tail -10
```

Expected: 3 passed.

- [ ] **Step 5: Add the runtime emitter task spawner**

Append to `crates/senseid/src/tasks/progress_emitter.rs`:

```rust
/// Public entry point — spawns the emitter as a tokio task that lives for the
/// daemon's lifetime. Subscribes to the task queue's TaskEvent stream and
/// publishes throttled StateEvent::folder_update events to the API broadcast channel.
pub fn spawn(
    task_events: broadcast::Receiver<TaskEvent>,
    state_events: broadcast::Sender<StateEvent>,
    pg: Arc<crate::db::pg_store::PgStore>,
) {
    tokio::spawn(run(task_events, state_events, pg));
}

async fn run(
    mut task_events: broadcast::Receiver<TaskEvent>,
    state_events: broadcast::Sender<StateEvent>,
    pg: Arc<crate::db::pg_store::PgStore>,
) {
    let mut trackers: HashMap<String, FolderTracker> = HashMap::new();

    while let Ok(evt) = task_events.recv().await {
        match evt {
            TaskEvent::FolderQueued { folder_path, files_total } => {
                // Look up the folder + project from DB on first observation.
                if let Some(t) = build_tracker(&pg, &folder_path, files_total).await {
                    trackers.insert(folder_path, t);
                }
            }
            TaskEvent::Completed { kind, folder_path, .. } if kind == "process_file" => {
                if let Some(t) = trackers.get_mut(&folder_path) {
                    t.files_completed += 1;
                    let now = Instant::now();
                    if t.should_emit(now) {
                        t.last_emit_at = now;
                        t.last_emit_count = t.files_completed;
                        let _ = state_events.send(StateEvent::folder_update(scan_folder_from(t, FolderStatus::Indexing)));
                    }
                }
            }
            TaskEvent::Completed { kind, folder_path, .. } if kind == "build_connections" => {
                if let Some(t) = trackers.remove(&folder_path) {
                    // Final emit — always fires regardless of throttle.
                    let _ = state_events.send(StateEvent::folder_update(scan_folder_from(&t, FolderStatus::Indexed)));
                }
            }
            _ => {}
        }
    }
}

async fn build_tracker(
    pg: &crate::db::pg_store::PgStore,
    folder_path: &str,
    files_total: u32,
) -> Option<FolderTracker> {
    let row = pg.get_repo_by_path(folder_path).await.ok().flatten()?;
    let folder_id   = row.get("id")?.as_str()?.to_string();
    let project_id  = row.get("project_id")?.as_str()?.to_string();
    let folder_name = row.get("name")?.as_str()?.to_string();
    Some(FolderTracker {
        folder_id, project_id, folder_name,
        path: folder_path.to_string(),
        kind: FolderKind::Git,
        stack: vec![],
        files_total,
        files_completed: 0,
        last_emit_at: Instant::now(),
        last_emit_count: 0,
    })
}

fn scan_folder_from(t: &FolderTracker, status: FolderStatus) -> ScanFolder {
    ScanFolder {
        id: t.folder_id.clone(),
        project_id: t.project_id.clone(),
        name: t.folder_name.clone(),
        path: t.path.clone(),
        kind: t.kind,
        stack: t.stack.clone(),
        files_total: t.files_total,
        files_completed: t.files_completed,
        status,
    }
}
```

If `PgStore::get_repo_by_path` doesn't exist, add it next to the existing `get_repo_by_name` in `crates/senseid/src/db/pg_store.rs`:

```rust
pub async fn get_repo_by_path(&self, abs_path: &str) -> Result<Option<serde_json::Value>, String> {
    let row: Option<(uuid::Uuid, uuid::Uuid, String)> = sqlx_core::query_as::query_as(
        "SELECT id, project_id, name FROM sensei.folders WHERE abs_path = $1 LIMIT 1"
    ).bind(abs_path).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(row.map(|(id, project_id, name)| serde_json::json!({
        "id": id, "project_id": project_id, "name": name,
    })))
}
```

- [ ] **Step 6: Build clean**

```bash
cargo build --features dev -p senseid 2>&1 | tail -5
```

Expected: clean build.

- [ ] **Step 7: Commit**

```bash
git add crates/senseid/src/tasks/progress_emitter.rs \
        crates/senseid/src/tasks/mod.rs \
        crates/senseid/src/db/pg_store.rs
git commit -m "feat(daemon): throttled folder_update emitter for scan pipeline"
```

---

### Task 5: Wire the progress emitter at daemon startup

**Files:**
- Modify: `crates/senseid/src/tasks/executor.rs`

The emitter needs to subscribe to the queue's `TaskEvent` broadcast and publish to the app's `event_tx` SSE channel. Both already exist on the `AppState`. The wiring lives wherever the task queue executor is bootstrapped.

- [ ] **Step 1: Find the executor bootstrap**

```bash
grep -n "TaskQueue::new\|spawn_workers\|fn start\|spawn(" crates/senseid/src/tasks/executor.rs | head -10
```

Note the call site where the queue and broadcast channels are wired. The progress_emitter::spawn call needs `task_events` (queue's broadcast Receiver), `state_events` (the API's event_tx Sender), and the PgStore Arc.

- [ ] **Step 2: Add the spawn call**

At the executor bootstrap site (typically `executor.rs` in a function called from `api/server.rs::start_server`), after the queue is created but before workers spawn, add:

```rust
crate::tasks::progress_emitter::spawn(
    queue.sender().subscribe(),
    state_events_tx.clone(),
    pg.clone(),
);
```

Adjust variable names to match the local bindings at the call site.

- [ ] **Step 3: Build clean**

```bash
cargo build --features dev -p senseid 2>&1 | tail -5
```

Expected: clean build.

- [ ] **Step 4: Smoke test via live daemon**

```bash
make install-dev && senseid-dev restart
# In another terminal, watch SSE
curl -N http://127.0.0.1:7745/api/scan/events &
SSE_PID=$!
# Trigger a scan against a small test repo
curl -X POST -H 'Content-Type: application/json' \
  -d '{"path":"/tmp/sensei-emitter-test"}' http://127.0.0.1:7745/api/scan/folder
# Wait a few seconds, then kill the SSE
sleep 5 && kill $SSE_PID
```

Expected: SSE log includes both `"entity":"folder","action":"add"` and several `"entity":"folder","action":"update"` events with monotonically increasing `filesCompleted`. If only `add` shows up, the emitter isn't subscribed correctly.

(Skip the live smoke if you don't have a test repo handy — the integration test in Task 9 covers it.)

- [ ] **Step 5: Commit**

```bash
git add crates/senseid/src/tasks/executor.rs
git commit -m "feat(daemon): spawn scan progress emitter at startup"
```

---

### Task 6: Emit project_update::active when all folders indexed

**Files:**
- Modify: `crates/senseid/src/tasks/progress_emitter.rs`
- Modify: `crates/senseid/src/db/pg_store.rs`

When the emitter sees the terminal `folder_update::Indexed`, it should check: are all folders for this project now indexed? If yes, emit `project_update` with `status=Active`.

- [ ] **Step 1: Add a helper to PgStore**

In `crates/senseid/src/db/pg_store.rs`, add:

```rust
pub async fn count_unindexed_folders(&self, project_id: uuid::Uuid) -> Result<i64, String> {
    let row: (i64,) = sqlx_core::query_as::query_as(
        "SELECT COUNT(*) FROM sensei.folders
          WHERE project_id = $1
            AND status NOT IN ('indexed', 'failed')"
    ).bind(project_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(row.0)
}
```

- [ ] **Step 2: Update the emitter to check after each terminal folder**

In `crates/senseid/src/tasks/progress_emitter.rs`, find the `build_connections` branch in `run()` and add the project-completion check:

```rust
TaskEvent::Completed { kind, folder_path, .. } if kind == "build_connections" => {
    if let Some(t) = trackers.remove(&folder_path) {
        let project_id_str = t.project_id.clone();
        let _ = state_events.send(StateEvent::folder_update(scan_folder_from(&t, FolderStatus::Indexed)));

        // Check whether this completion makes the whole project ready.
        if let Ok(project_uuid) = uuid::Uuid::parse_str(&project_id_str) {
            if let Ok(0) = pg.count_unindexed_folders(project_uuid).await {
                let _ = state_events.send(StateEvent::project_update(crate::api::events::ScanProject {
                    id: project_id_str.clone(),
                    name: String::new(),   // project_update payload — name carried from initial add
                    status: crate::api::events::ProjectStatus::Active,
                    folders: vec![],
                    auto_detected: true,
                    confidence: crate::api::events::Confidence::High,
                }));
            }
        }
    }
}
```

Note: the `project_update` event's `data` is merged into the existing client-side project entry by `ScanProjectState.update` (which has been merging by id since the wizard was first built). Empty fields don't overwrite — only `status` flips to `active` for the matching id.

Verify that behavior in `scan-state.svelte.ts:ScanProjectState.update`:

```typescript
const merged = { ...existing, ...patch };
```

This spread overwrites `name` with the empty string from the patch, which is wrong. Fix it: change the `name` field in the emitted update to use the existing tracked name. But we don't have the project name in the tracker — we'd need to add it. Simpler fix: filter empty strings in the merge.

Update `app/src/lib/scan-state.svelte.ts:ScanProjectState.update` to skip empty-string overwrites:

```typescript
const cleaned: Partial<ScanProject> = {};
for (const [k, v] of Object.entries(patch)) {
    if (v === '' || v === undefined) continue;
    (cleaned as any)[k] = v;
}
const merged = { ...existing, ...cleaned };
```

(Defer this client change to a separate file edit if it cleans up the diff, or include it in this task — the daemon change requires the client tolerance, so they belong together.)

- [ ] **Step 3: Run progress_emitter tests + build**

```bash
cargo test --features dev -p senseid progress_emitter 2>&1 | tail -5
cargo build --features dev -p senseid 2>&1 | tail -5
```

Expected: tests pass, clean build.

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/tasks/progress_emitter.rs \
        crates/senseid/src/db/pg_store.rs \
        app/src/lib/scan-state.svelte.ts
git commit -m "feat(daemon): emit project_update active when all folders indexed"
```

---

### Task 7: Drop substring filter from ScanActivityState

**Files:**
- Modify: `app/src/lib/scan-state.svelte.ts`
- Modify: `app/src/lib/scan-state.spec.svelte.ts`

- [ ] **Step 1: Update the test fixtures + assertions**

In `app/src/lib/scan-state.spec.svelte.ts`, find the `discovered counts discover + found` block. Rewrite as:

```typescript
it('discovered counts every discover-level event', () => {
    const state = new ScanActivityState();
    state.add(activity('a1', 'discover', '/code/lumen · git folder', 0.1));
    state.add(activity('a2', 'discover', '/code/lumen/app · git folder', 0.18));
    state.add(activity('a3', 'discover', '/code/canvas · standalone folder', 0.22));
    state.add(activity('a4', 'info', '3 git · 0 sibling · 1 standalone folders discovered', 0.30));
    expect(state.discovered).toBe(3);
});
```

For the `processed counts process + graph extracted` block — `processed` is moving off `ScanActivityState` entirely (see Task 8). Replace this block with:

```typescript
it('does not expose a processed counter — see ScanProjectState folder statuses', () => {
    const state = new ScanActivityState();
    expect((state as any).processed).toBeUndefined();
});
```

- [ ] **Step 2: Run the tests — verify they fail**

```bash
cd app && bunx vitest run scan-state 2>&1 | tail -10
```

Expected: failures pointing to the substring filter in `discovered` and the still-present `processed` getter.

- [ ] **Step 3: Apply the fix**

In `app/src/lib/scan-state.svelte.ts`:

```typescript
// ScanActivityState
get discovered() {
    return this.items.filter(e => e.level === 'discover').length;
}

get queued() {
    return this.items.filter(e => e.level === 'queue').length;
}

// Drop the `processed` getter entirely.
```

- [ ] **Step 4: Run tests — verify they pass**

```bash
cd app && bunx vitest run scan-state 2>&1 | tail -5
```

Expected: 0 failures.

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/scan-state.svelte.ts app/src/lib/scan-state.spec.svelte.ts
git commit -m "fix(app): scan discovered counter no longer relies on message substring"
```

---

### Task 8: Move processed counter onto ScanProjectState

**Files:**
- Modify: `app/src/lib/scan-state.svelte.ts`
- Modify: `app/src/lib/scan-state.spec.svelte.ts`
- Modify: `app/src/routes/(config)/setup/scan/+page.svelte`

- [ ] **Step 1: Add a failing test**

In `app/src/lib/scan-state.spec.svelte.ts`, add (next to other `ScanProjectState` tests):

```typescript
describe('ScanProjectState.processedFiles + processedFolders', () => {
    it('counts folders with status indexed across projects', () => {
        const projects = new ScanProjectState();
        projects.add({
            id: 'p1', name: 'Lumen', status: 'indexing', autoDetected: true, confidence: 'high',
            folders: [
                { id: 'f1', name: 'app',    path: '/code/lumen/app',    stack: ['rust'], filesTotal: 100, filesCompleted: 100, status: 'indexed' },
                { id: 'f2', name: 'canvas', path: '/code/lumen/canvas', stack: ['ts'],   filesTotal: 50,  filesCompleted: 25,  status: 'indexing' },
            ],
        });
        expect(projects.processedFolders).toBe(1);
    });
});
```

- [ ] **Step 2: Run — verify failure**

```bash
cd app && bunx vitest run scan-state 2>&1 | tail -10
```

Expected: error about `processedFolders` not defined.

- [ ] **Step 3: Add the getter**

In `app/src/lib/scan-state.svelte.ts`, inside `ScanProjectState` (near `readyFolders`):

```typescript
get processedFolders() {
    return this.items.reduce((s, p) => s + p.folders.filter(f => f.status === 'indexed').length, 0);
}
```

- [ ] **Step 4: Wire to the scan page stats bar**

In `app/src/routes/(config)/setup/scan/+page.svelte`, the stats bar (around line 181) currently reads:

```svelte
{ value: activities.processed, label: "PROCESSED" }
```

Change to:

```svelte
{ value: projects.processedFolders, label: "PROCESSED" }
```

- [ ] **Step 5: Run tests + svelte-check**

```bash
cd app && bunx vitest run scan-state 2>&1 | tail -5
cd app && bunx svelte-check 2>&1 | tail -3
```

Expected: tests pass, 0 type errors.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/scan-state.svelte.ts \
        app/src/lib/scan-state.spec.svelte.ts \
        app/src/routes/'(config)'/setup/scan/+page.svelte
git commit -m "fix(app): processedFolders derived from project folder statuses"
```

---

### Task 9: Scan lifecycle Rust integration test

**Files:**
- Create: `crates/senseid/tests/scan_lifecycle.rs`

This integration test boots a sensei_dev test DB, runs a scan against a temp directory with a known file count, subscribes to the broadcast channel, and asserts the event sequence.

- [ ] **Step 1: Create the integration test**

```rust
//! Scan lifecycle — exercises the daemon end-to-end via HTTP and asserts
//! the SSE event stream contains throttled folder_update events plus a
//! terminal project_update::active.
//!
//! Requires SENSEI_API_URL and a writable /tmp. Self-skips otherwise.

use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio_stream::StreamExt;

fn base_url() -> Option<String> { std::env::var("SENSEI_API_URL").ok() }

#[tokio::test]
async fn scan_emits_folder_progress_and_project_active() {
    let Some(url) = base_url() else { return; };
    let c = Client::new();

    // 1. Seed a tiny git folder with ~30 indexable files.
    let root = PathBuf::from(format!("/tmp/sensei-scan-lifecycle-{}", std::process::id()));
    fs::create_dir_all(root.join("repo")).unwrap();
    fs::create_dir_all(root.join("repo/.git")).unwrap();  // make it look like git
    for i in 0..30 {
        fs::write(root.join(format!("repo/file_{i:03}.rs")), "fn main() {}").unwrap();
    }

    // 2. Connect to SSE before triggering scan.
    let sse_resp = c.get(format!("{url}/api/scan/events")).send().await.unwrap();
    let mut sse_stream = sse_resp.bytes_stream();

    // 3. Trigger scan.
    let _ = c.post(format!("{url}/api/scan/folder"))
        .json(&serde_json::json!({ "path": root.display().to_string() }))
        .send().await.unwrap();

    // 4. Collect events for up to 30 seconds.
    let mut folder_adds = 0;
    let mut folder_updates_with_progress = 0;
    let mut folder_terminal_indexed = 0;
    let mut project_active = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    while tokio::time::Instant::now() < deadline {
        let res = tokio::time::timeout(Duration::from_secs(1), sse_stream.next()).await;
        let Ok(Some(Ok(chunk))) = res else { continue; };
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            let Some(json) = line.strip_prefix("data: ") else { continue; };
            let Ok(evt): Result<serde_json::Value, _> = serde_json::from_str(json) else { continue; };
            let entity = evt["entity"].as_str().unwrap_or("");
            let action = evt["action"].as_str().unwrap_or("");
            match (entity, action) {
                ("folder", "add")    => folder_adds += 1,
                ("folder", "update") => {
                    let status = evt["data"]["status"].as_str().unwrap_or("");
                    let done = evt["data"]["filesCompleted"].as_u64().unwrap_or(0);
                    if status == "indexed" {
                        folder_terminal_indexed += 1;
                    } else if done > 0 {
                        folder_updates_with_progress += 1;
                    }
                }
                ("project", "update") => {
                    if evt["data"]["status"].as_str() == Some("active") {
                        project_active += 1;
                    }
                }
                _ => {}
            }
        }
        if project_active >= 1 && folder_terminal_indexed >= 1 { break; }
    }

    fs::remove_dir_all(&root).ok();

    assert!(folder_adds >= 1,                  "expected ≥1 folder add, got {folder_adds}");
    assert!(folder_updates_with_progress >= 1, "expected ≥1 progress folder_update, got {folder_updates_with_progress}");
    assert!(folder_terminal_indexed >= 1,      "expected ≥1 terminal indexed folder_update, got {folder_terminal_indexed}");
    assert!(project_active >= 1,               "expected ≥1 project_update::active, got {project_active}");
}
```

- [ ] **Step 2: Boot the daemon with latest binaries**

```bash
make install-dev && senseid-dev restart
```

- [ ] **Step 3: Run the integration test**

```bash
SENSEI_API_URL=http://127.0.0.1:7745 cargo test --features dev -p senseid --test scan_lifecycle -- --nocapture
```

Expected: 1 passed. The test logs the event counts; verify they look sensible (~30 files / 25 throttle = 1-2 progress events plus a terminal).

If the test fails because some count is 0, trace which: if no progress events, the throttle config or wiring is off; if no terminal, the BuildConnections kind string mismatch in the emitter; if no project_active, the count_unindexed_folders helper returns wrong.

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/tests/scan_lifecycle.rs
git commit -m "test(daemon): scan lifecycle integration — folder_update + project_active"
```

---

### Task 10: Libraries stage filter — scanned-this-run repos only

**Files:**
- Modify: `app/src/lib/wizard-state.svelte.ts`

The current `refreshLibraries` pulls every lib from `/api/libs`. Filter to libs whose `repos[]` overlaps with the projects currently in `wizardState.projects.projects`.

- [ ] **Step 1: Find the refreshLibraries method**

```bash
grep -n "refreshLibraries\b" app/src/lib/wizard-state.svelte.ts
```

Read the existing implementation to understand the lib shape — `repos` is typically an array of repo names or ids.

- [ ] **Step 2: Update the filter**

In `app/src/lib/wizard-state.svelte.ts`, replace the body of `refreshLibraries`:

```typescript
async refreshLibraries(): Promise<void> {
    const api = senseiApi(appState.port);
    const fresh = await api.getLibs();

    // Set of repo names in projects discovered during THIS wizard run.
    // Filter libs to those touching at least one of these repos.
    const scannedRepoNames = new Set<string>();
    for (const p of this.projects.projects) {
        for (const f of p.folders) scannedRepoNames.add(f.name);
    }

    const previous = new Map(this.libraries.libs.map(l => [l.name, l.enabled]));
    this.libraries = {
        libs: fresh.libs
            .filter(l => (l.repos ?? []).some((r: string) => scannedRepoNames.has(r)))
            .map(l => ({
                id: l.id || l.name,
                name: l.name,
                ecosystem: l.ecosystem ?? '',
                version: l.version ?? null,
                description: l.description ?? null,
                pageCount: l.pageCount ?? 0,
                repos: l.repos ?? [],
                repoCount: l.repoCount ?? (l.repos?.length ?? 0),
                enabled: previous.get(l.name) ?? true,
            })),
    };
}
```

If the lib's `repos` field uses ids instead of names, swap the set type — check the daemon's `/api/libs` response shape first via `curl http://127.0.0.1:7745/api/libs | python3 -m json.tool | head -30`.

- [ ] **Step 3: Verify**

```bash
cd app && bunx svelte-check 2>&1 | tail -3
cd app && bunx vitest run 2>&1 | tail -5
```

Expected: 0 errors. Some existing tests may expect the unfiltered count — update those tests to set `wizardState.projects.projects` with at least one folder that matches the test libs.

- [ ] **Step 4: Commit**

```bash
git add app/src/lib/wizard-state.svelte.ts \
        app/src/lib/wizard-state.spec.svelte.ts  # if tests changed
git commit -m "fix(app): Libraries stage filters to repos scanned this run"
```

---

### Task 11: Done routing fix — surface commit errors

**Files:**
- Modify: `app/src/routes/(config)/+layout.svelte`
- Modify: `app/src/lib/wizard-state.svelte.ts`

- [ ] **Step 1: Make commitStage("done") throw on failure**

In `app/src/lib/wizard-state.svelte.ts`, find `commitStage` (around line 450). Currently:

```typescript
async commitStage(stageId: string): Promise<boolean> {
    const handler = COMMIT_HANDLERS[stageId];
    if (!handler) return true;
    try {
        await handler(this, api);
        await api.setConfig({ [`setup.${stageId}`]: 'done' });
        this.completion[stageId] = 'done';
        return true;
    } catch { return false; }
}
```

Add a special branch for `done`:

```typescript
async commitStage(stageId: string): Promise<boolean> {
    const handler = COMMIT_HANDLERS[stageId];
    if (!handler) return true;
    try {
        await handler(this, api);
        await api.setConfig({ [`setup.${stageId}`]: 'done' });
        this.completion[stageId] = 'done';
        return true;
    } catch (e) {
        // The done stage flips setup_complete on the daemon — its failure
        // must propagate, not silently fall back to false-then-navigate.
        if (stageId === 'done') throw e;
        return false;
    }
}
```

- [ ] **Step 2: Surface the error inline in the layout**

In `app/src/routes/(config)/+layout.svelte`, add to the script block (near other `let ... = $state(...)` declarations):

```svelte
let commitError = $state<string | null>(null);
```

Replace the `next()` function with:

```svelte
async function next() {
    if (committing) return;
    if (!canAdvance) return;

    committing = true;
    commitError = null;
    try {
        if (isLast) {
            await wizardState.commitStage("done");
            // Force a re-read of appState.config so reroute sees setup_complete=1
            // before goto fires. The optimistic update in setSetupComplete handles
            // the in-memory case, but the load() guards against any race.
            await appState.load();
            goto("/");
            return;
        }
        const ok = await wizardState.commitStage(stage.id);
        if (ok) {
            const path = nextStagePath(page.url.pathname);
            if (path) goto(path);
        }
    } catch (e) {
        commitError = e instanceof Error ? e.message : String(e);
    } finally {
        committing = false;
    }
}
```

Add an inline error display just above the bottom-nav (around line 183, before the `<!-- Bottom nav -->` comment):

```svelte
{#if commitError}
    <div class="mx-16 mb-2 p-3 rounded-md border border-danger-z5 bg-surface-z2 text-xs text-danger-z5">
        Could not finish: {commitError} — fix and try Continue again.
    </div>
{/if}
```

- [ ] **Step 3: Add a test for the failure path**

In `app/src/lib/wizard-state.spec.svelte.ts`, add:

```typescript
it('commitStage("done") propagates errors instead of returning false', async () => {
    const ws = new WizardState();
    // Stub the api to make setSetupComplete reject.
    const originalApi = (await import('./api.js')).senseiApi;
    vi.spyOn(await import('./api.js'), 'senseiApi').mockImplementation(((_port: number) => ({
        ...originalApi(0),
        trySetConfig: () => Promise.resolve({ ok: false, error: { status: 500, message: 'boom' } }),
    })) as any);

    await expect(ws.commitStage('done')).rejects.toThrow();
});
```

- [ ] **Step 4: Verify**

```bash
cd app && bunx svelte-check 2>&1 | tail -3
cd app && bunx vitest run wizard-state 2>&1 | tail -5
```

Expected: 0 type errors, new test passes.

- [ ] **Step 5: Commit**

```bash
git add app/src/routes/'(config)'/+layout.svelte \
        app/src/lib/wizard-state.svelte.ts \
        app/src/lib/wizard-state.spec.svelte.ts
git commit -m "fix(app): Done stage commit errors surface inline, no nav on failure"
```

---

### Task 12: Restore the setup gate + un-skip gate test

**Files:**
- Modify: `app/src/hooks.ts`
- Modify: `app/src/hooks.spec.svelte.ts`

This is the final code change before E2E verification. **Do not run this task until all previous tasks have shipped and the live wizard works end-to-end.**

- [ ] **Step 1: Restore the setup gate**

In `app/src/hooks.ts`, replace the TEMP block:

```typescript
// TEMP: setup gate disabled while debugging Phase 0 in the live app (2026-05-27).
// Restore once setup wizard regressions tracked in docs/backlog.md are resolved.
// if (!ALWAYS_REACHABLE.has(path) && !path.startsWith("/setup") && !wizardState.isOk) return "/setup/welcome";
void wizardState;
```

with the original logic:

```typescript
if (!ALWAYS_REACHABLE.has(path) && !path.startsWith("/setup") && !wizardState.isOk) return "/setup/welcome";
```

- [ ] **Step 2: Un-skip the gate test**

In `app/src/hooks.spec.svelte.ts`, find:

```typescript
// TEMP: setup gate disabled in hooks.ts while debugging Phase 0 in the
// live app. Restore both this test and the gate together once the setup
// wizard regressions in docs/backlog.md are resolved.
it.skip('redirects to /setup/welcome when setup is incomplete', () => {
```

Change `it.skip` back to `it` and remove the three TEMP comment lines.

- [ ] **Step 3: Verify**

```bash
cd app && bunx vitest run hooks 2>&1 | tail -5
```

Expected: 10 tests pass (the previously-skipped one is back in the green count).

- [ ] **Step 4: Run the full suite**

```bash
cd app && bunx vitest run 2>&1 | tail -5
```

Expected: all green; total goes up by 1 from previous baseline.

- [ ] **Step 5: Commit**

```bash
git add app/src/hooks.ts app/src/hooks.spec.svelte.ts
git commit -m "feat(app): restore setup gate now that wizard regressions are fixed"
```

---

### Task 13: Happy-path E2E spec

**Files:**
- Create: `app/e2e/tests/setup-wizard-happy-path.spec.ts`

This spec walks through all 10 stages with a seeded scan root and verifies the wizard lands in Observatory.

- [ ] **Step 1: Create the spec**

```typescript
/**
 * End-to-end walk-through of the 10-stage setup wizard.
 *
 * Seeds a tiny git folder, drives the wizard from welcome → done, asserts:
 *  - Scan stage progress bars animate.
 *  - Projects stage shows the seeded project after scan.
 *  - Done → Continue navigates to /, NOT back to /setup/welcome.
 *  - Fresh app load with setup_complete=1 lands directly in Observatory.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';
import { promises as fs } from 'fs';

async function resetSetupKeys(): Promise<void> {
    const keys = [
        'setup.welcome', 'setup.preferences', 'setup.assistants',
        'setup.roots', 'setup.scan', 'setup.projects', 'setup.libraries',
        'setup.instruments', 'setup.inference', 'setup.done',
        'setup_complete',
    ];
    for (const k of keys) {
        await fetch(`${DAEMON_URL}/api/config/${k}`, { method: 'DELETE' });
    }
}

async function seedTinyRepo(): Promise<string> {
    const root = `/tmp/sensei-wizard-e2e-${Date.now()}`;
    await fs.mkdir(`${root}/.git`, { recursive: true });
    for (let i = 0; i < 5; i++) {
        await fs.writeFile(`${root}/file_${i}.rs`, 'fn main() {}');
    }
    return root;
}

test('setup wizard end-to-end happy path', async ({ tauriPage }) => {
    await resetSetupKeys();
    const repoRoot = await seedTinyRepo();

    await navigateTo(tauriPage, '/setup/welcome');

    // Walk forward stage-by-stage. The Continue button text varies; click by
    // role + visible text + position.
    async function clickContinue() {
        await tauriPage.locator('button[data-testid="continue-button"], button.btn-primary').last().click();
        await tauriPage.waitForTimeout(500);
    }

    // Welcome
    await clickContinue();
    // Preferences — fill displayName
    await tauriPage.locator('input[name="displayName"]').fill('e2e-user');
    await clickContinue();
    // Assistants — accept defaults
    await clickContinue();
    // Roots — add the seeded repo
    await tauriPage.locator('button:has-text("Add root")').click();
    await tauriPage.locator('input[type="text"]').last().fill(repoRoot);
    await tauriPage.locator('button:has-text("Save")').click();
    await clickContinue();
    // Scan — wait for indexing to complete
    await expect(tauriPage.locator('text=PROCESSED').locator('..')).toContainText(/[1-9]/, { timeout: 30_000 });
    await clickContinue();
    // Projects → Libraries → Instruments → Inference → Done
    for (let i = 0; i < 4; i++) await clickContinue();

    // On Done — final Continue should land us on /
    await tauriPage.locator('button:has-text("Enter observatory")').click();
    await tauriPage.waitForURL('**/', { timeout: 5_000 });

    // Confirm we're NOT bouncing back to /setup/welcome.
    await tauriPage.waitForTimeout(1000);
    expect(tauriPage.url()).not.toContain('/setup');
});
```

- [ ] **Step 2: Boot daemon + run**

```bash
make install-dev && senseid-dev restart
cd app && bunx playwright test --config e2e/playwright.config.ts --grep "setup wizard end-to-end" 2>&1 | tail -10
```

Expected: 1 passed. If the test fails on Scan completion timeout, the daemon-side emitter isn't producing terminal events; retrace Tasks 4–6.

- [ ] **Step 3: Commit**

```bash
git add app/e2e/tests/setup-wizard-happy-path.spec.ts
git commit -m "test(e2e): setup wizard end-to-end happy path"
```

---

### Task 14: Final zero-errors sweep + push

**Files:** (verification only)

- [ ] **Step 1: Full Rust sweep**

```bash
cargo test --features dev 2>&1 | grep -E "^test result|FAILED" | tail -20
```

Expected: every line shows ok, 0 failed.

- [ ] **Step 2: Clippy strict**

```bash
cargo clippy --features dev --all-targets -- -D warnings 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 3: App sweep**

```bash
cd app && bunx svelte-check && bunx vitest run && bunx playwright test --config e2e/playwright.config.ts 2>&1 | tail -15
```

Expected: 0 type errors, all unit + E2E tests pass.

- [ ] **Step 4: Manual smoke — fresh setup flow**

```bash
make install-dev && senseid-dev restart
# Reset all setup state in the daemon
for k in welcome preferences assistants roots scan projects libraries instruments inference done; do
    curl -X DELETE "http://127.0.0.1:7745/api/config/setup.${k}"
done
curl -X DELETE "http://127.0.0.1:7745/api/config/setup_complete"
make app-dev-bundle
```

In the app, walk through the wizard. Confirm:
- All 10 stages render.
- Scan progress bars animate from 0% to 100%.
- Projects stage shows the project after scan.
- Done → Continue lands you in Observatory (not back at Welcome).
- Force-quit and relaunch — second launch should go straight to Observatory.

- [ ] **Step 5: Push develop + merge to main**

```bash
git push origin develop
git checkout main
git pull --ff-only origin main
git merge --no-ff develop -m "Merge branch 'develop' — setup wizard rehab"
git push origin main
git checkout develop
git merge --no-ff main -m "Merge branch 'main' back into develop"
git push origin develop
```

---

## Self-review

**Spec coverage** — each spec section maps to a task:

| Spec section | Task(s) |
|---|---|
| Bucket A: Daemon progress emitter | 4, 5, 6 |
| Bucket A: Client scan-state refactor | 7, 8 |
| Bucket A: Integration test | 9 |
| Bucket B: Libraries filter | 10 |
| Bucket C: Switch + card-selected tokens | 2 |
| Bucket C: Drop rail disabled + Instruments lag | 3 |
| Bucket D: Drop Assignments | 1 |
| Bucket D: Done routing fix | 11 |
| Restore gate + un-skip test | 12 |
| Happy-path E2E + final sweep | 13, 14 |

No spec gaps.

**Placeholder scan** — no TBD/TODO/implement-later patterns in any task body. The TODO in the Background description refers to the literal `TODO` comment in the deleted Assignments stub.

**Type consistency:**
- `FolderTracker` fields match between unit test and runtime emitter.
- `ScanFolder` field names match between daemon (snake_case in Rust, camelCase via serde rename) and TS interface (`filesCompleted`, `filesTotal`, `projectId`).
- `ProjectStatus::Active` ↔ TS `'active'` — both serialize lowercase.
- `processedFolders` getter name consistent across spec, test, page binding.
- `commitError` state name consistent in `+layout.svelte` and the inline display block.

No drift.

---

*End of plan.*
