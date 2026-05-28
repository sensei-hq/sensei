# Setup Wizard Rehab Design

**Date:** 2026-05-27
**Status:** Draft — awaiting user review
**Scope:** Restore the setup wizard to a working end-to-end first-run flow. Bundled rehab of seven bugs surfaced by the live smoke test in `docs/backlog.md`, collapsed into four root-cause buckets.

---

## Background

The Phase 0 knowledge plane shipped cleanly, but a live smoke test of `make app-dev-bundle` exposed seven setup-wizard bugs and a scan-pipeline regression (separately diagnosed in `docs/backlog.md`). This spec gathers them into one rehab feature so the wizard ships as a coherent first-run experience.

The wizard ran in this shape pre-regression — the scan pipeline contracts drifted, several visual affordances are unfinished, one stage is a literal TODO stub, and the Done-stage Continue button silently loses state.

The setup-gate disable from this morning (`hooks.ts` patch + skipped gate test) is treated as a TEMP measure. The rehab restores both at the end.

## Vocabulary

| Term | Meaning |
|---|---|
| Stage | One step in the wizard (10 post-rehab: Welcome, Preferences, Assistants, Roots, Scan, Projects, Libraries, Instruments, Inference, Done). |
| Rail | The left-side stage list in `(config)/+layout.svelte`. Shows where you are and what's done. |
| Active | The currently-routed stage. Tracked via `wizardState.setActive(id)` from the layout's `$effect`. |
| Done (status) | A stage whose commit handler has run successfully and the daemon has the corresponding `setup.<id>` config key. |
| Progress emitter | New helper (Bucket A) that throttles per-file folder_update events to 300ms or 25 files. |

## Bug bucket map

The seven backlog bugs collapse into four buckets:

| Bucket | Bugs | Layer |
|---|---|---|
| **A. Scan pipeline** | Scan static (#2), Projects empty (#3), Done "no folders scanned" (#7-stat) | Rust daemon + app state |
| **B. Stale persistent data** | Libraries shows 130 stale (#4) | App / data filter |
| **C. UI affordance + routing** | Assistants Switch unclear (#1), Instruments rail rendering lag + active-marker desync (#5) | App CSS + a11y + SvelteKit routing |
| **D. Stub + Done routing** | Assignments empty stub (#6), Done → Welcome (#7-route) | App only |

## Architecture

All daemon work lives inside the existing scan/process task handlers plus one new helper module. No new endpoints. No schema changes. The SSE envelope shape (`StateEvent { action, entity, data }`) is unchanged — clients just receive more of them.

```
┌─ Daemon ─────────────────────────────────────────────────────────────┐
│  scan.rs        — discover activities (kept; copy unchanged)          │
│  process.rs     — initial folder_add + project_add (kept)            │
│                                                                       │
│  progress_emitter.rs (NEW)                                            │
│    throttle: 300ms OR 25 files, whichever comes first                │
│    emits StateEvent::folder_update with files_completed              │
│    finalize() always fires regardless of throttle                    │
│                                                                       │
│  process.rs (after all file-tasks complete for a folder)              │
│    - emit folder_update with status=indexed, files_completed=N        │
│    - if all folders for a project are indexed → project_update active │
└────────────────────────┬─────────────────────────────────────────────┘
                         │ SSE (existing /api/scan/events)
┌────────────────────────▼─────────────────────────────────────────────┐
│  scan-state.svelte.ts                                                 │
│    discovered: count by level (drop "found" substring filter)        │
│    processed:  derived from ScanProjectState folder statuses         │
│    folder progress: read filesCompleted from update events           │
│  scan/+page.svelte                                                    │
│    keeps poll for "queue idle" terminal signal (event-driven         │
│    progress, polling for completion handles watcher residue)         │
│  Other stages: targeted fixes per bucket                              │
└──────────────────────────────────────────────────────────────────────┘
```

## Bucket A — Scan pipeline

### Daemon side

**New module `crates/senseid/src/tasks/progress_emitter.rs`:**

```rust
pub struct ProgressEmitter {
    folder_id:   String,
    project_id:  String,
    folder_name: String,
    path:        String,
    kind:        FolderKind,
    stack:       Vec<String>,
    files_total: u32,
    files_done:  AtomicU32,
    last_emit:   Mutex<(Instant, u32)>,   // (when, files_done at that emit)
    tx:          broadcast::Sender<StateEvent>,
}

impl ProgressEmitter {
    pub fn record_file(&self) {
        let now_done = self.files_done.fetch_add(1, Ordering::Relaxed) + 1;
        let mut guard = self.last_emit.lock().unwrap();
        let (last_t, last_count) = *guard;
        let elapsed = last_t.elapsed();
        if elapsed >= Duration::from_millis(300) || now_done - last_count >= 25 {
            *guard = (Instant::now(), now_done);
            drop(guard);
            self.emit_update(now_done, FolderStatus::Indexing);
        }
    }

    pub fn finish(&self, status: FolderStatus) {
        let final_count = self.files_done.load(Ordering::Relaxed);
        self.emit_update(final_count, status);
    }

    fn emit_update(&self, completed: u32, status: FolderStatus) {
        let _ = self.tx.send(StateEvent::folder_update(ScanFolder {
            id: self.folder_id.clone(),
            project_id: self.project_id.clone(),
            name: self.folder_name.clone(),
            path: self.path.clone(),
            kind: self.kind,
            stack: self.stack.clone(),
            files_total: self.files_total,
            files_completed: completed,
            status,
        }));
    }
}
```

**`process.rs` integration:**

1. After the initial `folder_add` (line ~64) — construct a `ProgressEmitter` and thread it into the file-task spawning loop.
2. When a file's indexing task completes — call `emitter.record_file()`.
3. After the join-all on all file tasks for a folder — call `emitter.finish(FolderStatus::Indexed)`.
4. New helper `maybe_emit_project_active(project_id, ctx)` — query `sensei.folders` for the project's folders; if every folder has `indexed` status (or terminal `failed`), emit `StateEvent::project_update` with `status=Active`.

The file-task spawning needs the emitter passed by reference. Wrap in `Arc<ProgressEmitter>` so concurrent file tasks can record without a mutex collision.

### Client side

**`app/src/lib/scan-state.svelte.ts`:**

```typescript
// ScanActivityState
get discovered() {
    return this.items.filter(e => e.level === 'discover').length;
}
// processed moves OFF ScanActivityState — it depends on folder status.
// Expose it via a derived state in scan/+page.svelte instead:
//   const processed = $derived(projects.items
//       .flatMap(p => p.folders)
//       .filter(f => f.status === 'indexed').length);
```

`ScanProjectState.applyFolder` already merges `folder_update` events correctly — no change needed there. The `filesCompleted` value will flow into the progress bar reactively.

**Activity messages — no copy change required.** The substring filter is gone; the existing message format stays.

## Bucket B — Libraries stale data

Filter the Libraries stage display to libs with at least one repo association in the currently-scanned roots. The daemon's libs table already records `repos[]` per lib. The fix:

- `wizard-state.svelte.ts:refreshLibraries()` already calls `api.getLibs()`. After fetching, additionally fetch the current run's repos (`/api/projects/...` for each project in `wizardState.projects.projects`) and filter the libs list to those whose `repos` array contains at least one repo that maps back to a project in the wizard's `projects` state.
- The Done stage summary count (`libsWrapped`) reads from `wizardState.libraries.libs.filter(l => l.enabled).length` — naturally drops when the list itself drops.

This is a read-time filter — no schema migration. Reverting the filter just shows all known libs again.

## Bucket C — UI affordance + routing

### Assistants Switch token fix

`app/src/lib/components/Switch.svelte`:

```svelte
<style>
    .switch.on {
        /* was: background: oklch(var(--color-primary-600) / 1);  — token undefined */
        background: oklch(var(--color-primary-z5) / 1);
    }
</style>
```

`app/src/routes/(config)/setup/assistants/+page.svelte`:

```svelte
<style>
    .card-selected {
        /* was: border: 1.5px solid oklch(var(--color-surface-z6) / 1); */
        border: 2px solid oklch(var(--color-primary-z5) / 1);
        background: oklch(var(--color-surface-z2) / 1);
    }
</style>
```

### Rail navigation — drop disabled state + investigate rendering lag

`app/src/routes/(config)/+layout.svelte`:

```svelte
<!-- Was: disabled={!isNavigable}, conditional onclick -->
<button
    data-rail-item
    data-stage-id={s.id}
    data-active={s.active}
    class="..."
    class:active={s.active}
    class:done={isDone}
    onclick={() => goto(s.path)}
>
    ...
</button>
```

Every rail item is always clickable. The Continue button at the bottom remains the gate (`disabled={!canAdvance || committing}`).

**Investigation step (must happen during implementation):** The reported symptoms — header updates but body lags by one step, active-marker doesn't follow back-navigation — suggest one of:

1. **Per-page component state retention.** Svelte 5 + SvelteKit reuses layouts across child route changes. If a stage's +page.svelte holds component-local `$state` that isn't reset on URL change, content can linger.
2. **`$effect`-driven active-flag lag.** The layout's `$effect(() => wizardState.setActive(stage.id))` runs after the URL settles; if any +page.svelte reads `wizardState.activeStage` synchronously during mount, it sees stale.
3. **Wizard-state mutation pattern.** `setActive` iterates and mutates `s.active = …` on each stage. If the array reference doesn't change, Svelte's reactivity may miss the update in some places.

The plan must include a focused step that loads `make app-dev-bundle`, reproduces the exact symptom, identifies which of the three is the cause, and fixes it. The disabled-state removal happens in the same patch.

## Bucket D — Stub + Done routing

### Drop Assignments stage

Files touched:
- `app/src/routes/(config)/stages.ts` — remove the `assignments` entry from `STAGES`.
- `app/src/routes/(config)/setup/assignments/` — delete the directory.
- `app/src/lib/wizard-state.svelte.ts` — remove from `COMMIT_HANDLERS` if present; from `canAdvance` switch if present; from `hydrate` completion map handling if present.
- E2E and unit tests referencing the stage — update to expect 10 stages, not 11.

The wizard goes 11 → 10. Existing users who have `setup.assignments = 'done'` in daemon config are harmless (the entry is just ignored).

### Done → Welcome routing fix

`app/src/routes/(config)/+layout.svelte:43-60`:

```svelte
async function next() {
    if (committing) return;
    if (!canAdvance) return;

    committing = true;
    try {
        if (isLast) {
            await wizardState.commitStage("done");
            // commitStage throws on failure; if we reach here it succeeded.
            // The optimistic appState update + the daemon write are both done.
            // Force a re-read of appState.config so reroute sees setup_complete=1.
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
        // Surface inline instead of swallowing — important for the Done stage
        // where a failed commit leaves the user stranded if we navigate anyway.
        commitError = e instanceof Error ? e.message : String(e);
    } finally {
        committing = false;
    }
}
```

And `wizardState.commitStage` should throw on the `done` stage failure rather than returning `false` — so the catch above actually fires.

Add a `commitError = $state<string | null>(null);` plus an inline error display block above the bottom-nav so the user sees what went wrong.

## Build sequence

Read as a dependency DAG:

1. **Drop Assignments stage** (isolated). 1 commit.
2. **Switch + card-selected token fix** (visual, isolated). 1 commit.
3. **Rail nav: drop disabled + investigate Instruments rendering lag** (1 commit, includes the rendering-lag fix).
4. **Daemon scan emitter** (Rust, includes unit tests for throttle).
5. **Daemon emit `project_update::active`** (Rust). Depends on (4).
6. **Client scan-state refactor** (TypeScript). Depends on (4–5).
7. **Libraries stage filter** (TypeScript).
8. **Done stage routing fix** (TypeScript).
9. **Restore setup gate + un-skip gate test** (TypeScript). Depends on every prior step landing.
10. **Manual smoke + E2E happy-path spec** (verification).

Daemon vs app work can interleave; (4)/(5) gate (6); (9) gates the final merge.

## Testing strategy

### Rust unit tests

- `progress_emitter.rs` — `record_file` cadence under `tokio::time::pause`/`advance`. Assert first emit after 300ms; 26th emit at file 25 if all within 300ms; `finish()` always fires regardless of throttle window.
- `process.rs` — exposed test that creates a synthetic project + folders, drives file-task completion, asserts the broadcast channel receives the expected sequence of `folder_update` events and a terminal `project_update`.

### Rust integration test

`crates/senseid/tests/scan_lifecycle.rs` (new): boot a sensei_dev test DB, run scan against a temp directory with a known file count, subscribe to the broadcast channel, assert `folder_update` events arrive with monotonically increasing `files_completed`, terminal `folder_update` has `status=indexed`, and a final `project_update` flips the project to `active`.

### TypeScript unit tests

- `scan-state.spec.svelte.ts` — update `discovered counts discover + found` to remove the "found" substring assertion; assert counts ALL discover-level events. Update the `processed` tests to drive folder status into `ScanProjectState` and read the count from a derived getter in the test fixture.
- New test: `commit failure on Done stage does not navigate and surfaces the error in commitError`.

### Playwright E2E

New `app/e2e/tests/setup-wizard-happy-path.spec.ts`: walk through all 10 stages with a seeded scan root (e.g., `/tmp/sensei-e2e-wizard-N/` containing a small fake repo). Assert:

- Scan stage: progress bars animate (poll DOM, assert `filesCompleted > 0` then `= filesTotal`, then folder status flips to `indexed`).
- Projects stage: shows the seeded project.
- Done stage: Continue button navigates to `/` (Observatory) and stays there — does NOT bounce back to Welcome.
- Subsequent app launch (simulated via `appState.load()` returning `setup_complete: '1'`) lands directly in Observatory.

Existing `inference-stage.spec.ts` and `boot-flow.spec.ts` need to be updated to drop any Assignments references and to expect the new 10-stage sequence.

## Error handling

| Error class | Daemon behavior | Client behavior |
|---|---|---|
| `setup_complete` daemon write fails on Done | 500 from PUT /api/config | Inline error card above the bottom-nav; Continue re-enables; no navigation |
| Scan emitter overflow (broadcast channel full) | Oldest dropped silently (existing channel semantics) | Activity log may have gaps; terminal state still correct because finalize always emits |
| Daemon unreachable mid-scan | n/a | Existing pattern: poll sets `daemonReachable=false`; warning surfaces; recovers when poll succeeds |
| Rail navigation to an incomplete stage | n/a | The stage's +page.svelte renders empty/partial state, which is fine |
| Stage page mounts before appState loaded | n/a | `+layout.ts` already throws 503; existing health gate handles this |

## Out of scope (this rehab)

- Building out Assignments — deferred to Observatory/Settings once sessions data exists.
- Architectural changes to wizard structure (rethinking the rail-as-progress-indicator, supporting multi-pass setup, etc.).
- Migrating the persistent Libraries DB to be session-scoped (we filter at read-time, not schema change).
- Scan-time integration with the knowledge plane (`/api/knowledge/*` is independent).

## Open questions (resolved during writing)

- **Q: Keep polling for scan completion or replace with an event?**
  A: Keep polling. Watchers continue to land file changes after initial scan; polling for queue-idle handles this naturally.
- **Q: Per-file emit vs throttled batch?**
  A: Throttled batch — 300ms or 25 files, whichever comes first (per user choice).
- **Q: Drop the disabled-state on rail items?**
  A: Yes. The disabled-until-done flow was masking the real Instruments rendering lag bug.
- **Q: Build Assignments or drop it?**
  A: Drop from wizard. Move to Settings once meaningful (separate work).
- **Q: Libraries stale data — filter at read-time or schema change?**
  A: Read-time filter (`b1` in design notes). Schema change is overkill for first-run.

---

*End of spec. Implementation follows the build sequence above; final commit (step 9) restores the setup gate and un-skips the gate test — this rehab is "done" only when the gate is back on and the happy-path E2E spec passes.*
