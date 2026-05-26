# Health State — Phase 1c Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal (narrow scope):** Connect Svelte `HealthState` to the Tauri sidecar's new `health_check` / `health_check_and_resolve` commands and the `health` event channel. **Nothing else.** The legacy `bootstrap.ts` exports stay in place (dormant — no caller after this phase). Setup wizard, update.rs, and any aux-command retirement are explicitly out of scope.

**End state after Phase 1c:** cold-deleted system → launch app → `/health` page walks `checking → resolving → ok` live, driven by real Rust check + resolve calls flowing through `HealthTransport` into `HealthState.applyEvent`.

**Architecture in one paragraph:** A new `HealthTransport` interface in `app/src/lib/health-transport.ts` (with `RealTransport` and `MockTransport` impls) sits between `HealthState` and Tauri. `HealthState.init()` calls `transport.check()` then conditionally `transport.resolve(onEvent)`. SessionStorage cache (`sensei:health = 'ready'`) is written from inside `apply()` when `status === 'ok'` so `hooks.client.ts` can read it synchronously during reroute. `appState.healthReady` / `setHealthReady` are removed.

**Tech stack:** TypeScript, Svelte 5, Tauri 2, vitest 4.

**Source contract:** `app/src/lib/health-types.ts` (TS side, finalized in 1a) + `crates/bootstrap/src/health/types.rs` (Rust side, finalized in 1b — matching shape via serde camelCase).

**Predecessors:**
- `docs/superpowers/specs/2026-05-12-health-state-design.md` (design)
- `docs/superpowers/plans/2026-05-12-health-state-phase-1a.md` (HealthState class + UI)
- `docs/superpowers/plans/2026-05-13-health-state-phase-1b.md` (Rust health surface)

---

## Discipline

Same rules as Phase 1a and 1b:

1. **TDD red-green-refactor at every step.** Tests first.
2. **100% line + branch coverage** on every new Phase-1c module via `vitest --coverage`. Existing 275 app tests must continue to pass.
3. **Mock transport drives most tests.** RealTransport (Tauri-bound) is exercised via integration smoke at the end.
4. **No regressions to the public payload contract.** The wire shape between Rust and TS was pinned in 1b's `tests/json_wire_shape.rs` — Phase 1c consumes that shape unchanged.

---

## Pre-flight

- [ ] **Confirm baseline**
  ```bash
  cd /Users/Jerry/Developer/sensei-hq/sensei
  git status --short
  cd app && bun run test:unit 2>&1 | tail -3   # 275 pass
  bun run check 2>&1 | tail -3                 # 0/0
  ```

- [ ] **Find every caller of the legacy bootstrap.ts surface**
  ```bash
  grep -rn "checkAndFixBootstrap\|listenBootstrapEvents\|listenBootstrapReport\|getDaemonPort\|getPlatform\|hasTauri\|BootstrapEvent\|BootstrapReport" app/src 2>&1 | head -30
  ```
  Expected: hits in `app/src/lib/bootstrap.ts` (the module itself), the now-stub `commands/update.rs` reference in TS (if any), maybe one or two stragglers. Catalog them — every caller migrates in this phase.

---

## Section A — Define the `HealthTransport` interface + MockTransport

The transport sits between `HealthState` and the wire (Tauri commands / events). One interface, two impls.

### Task A1 — Interface + MockTransport for tests

**Files:**
- Create: `app/src/lib/health-transport.ts`
- Create: `app/src/lib/health-transport.spec.svelte.ts`

**Interface (lifts the design from Section 2 of the spec):**
```ts
export interface HealthTransport {
  /** Sync fast path (daemon /health under the hood). Returns the current state. */
  check(): Promise<HealthPayload>;
  /** Streaming resolve. The provided callback fires for every HealthEvent. Resolves
   *  with the terminal HealthPayload (also delivered as a final `report` event). */
  resolve(current: HealthPayload, onEvent: (ev: HealthEvent) => void): Promise<HealthPayload>;
}
```

**MockTransport — scripted sequences for tests:**
```ts
export class MockTransport implements HealthTransport {
  /** Construct with a fixed check() return and a scripted event sequence for resolve(). */
  constructor(opts: {
    checkPayload: HealthPayload;
    resolveEvents?: HealthEvent[];
    resolveTerminal?: HealthPayload;     // defaults to checkPayload
  }) { ... }
  // Records every method call for assertions: this.checkCalls / this.resolveCalls
}
```

Tests must cover (10+ unit tests):
- `check()` returns the configured payload.
- `resolve(current, onEvent)` invokes `onEvent` once per scripted event, in order.
- `resolve` resolves with the terminal payload.
- Call recorder works (length, args).

### Task A2 — RealTransport (Tauri-bound)

**Files:**
- Modify: `app/src/lib/health-transport.ts` (append `RealTransport`)

```ts
export class RealTransport implements HealthTransport {
  async check(): Promise<HealthPayload> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<HealthPayload>('health_check');
  }
  async resolve(current: HealthPayload, onEvent: (ev: HealthEvent) => void): Promise<HealthPayload> {
    const { invoke } = await import('@tauri-apps/api/core');
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<HealthEvent>('health', (e) => onEvent(e.payload));
    try {
      await invoke<void>('health_check_and_resolve');
      // Wait for the terminal `report` event by stashing it and resolving when it arrives.
      // Implementation: a Promise<HealthPayload> that resolves on the first kind:'report' event.
      // The wrapper around onEvent captures `report` and resolves.
      return await new Promise<HealthPayload>((resolveFn) => {
        // wire a guard inside the onEvent path; details fleshed out in the task
      });
    } finally {
      unlisten();
    }
  }
}
```

Note: the `RealTransport.resolve` mechanic needs careful handling — `health_check_and_resolve` is fire-and-forget, so we listen for the terminal `report` event to know when it's done. Spec'd in detail when the task is executed.

Tests for RealTransport are integration-style (vi.mock the Tauri API imports). Coverage 100%.

---

## Section B — `HealthState.init()` / `HealthState.verify()` lifecycle

The HealthState class gains its missing lifecycle methods. The transport is injected via constructor (defaulting to `RealTransport` for production, `MockTransport` for tests).

### Task B1 — Constructor takes a HealthTransport

**Files:**
- Modify: `app/src/lib/health-state.svelte.ts`
- Modify: `app/src/lib/health-state.spec.svelte.ts`

```ts
export class HealthState {
  #transport: HealthTransport;
  #initPromise: Promise<void> | null = null;
  // …existing fields…
  constructor(seed: HealthPayload = emptyPayload, transport: HealthTransport = new RealTransport()) {
    this.#transport = transport;
    this.apply(seed);
  }
}
```

Tests: existing 30+ HealthState tests pass after adding a `new HealthState(emptyPayload, new MockTransport(...))` constructor variant.

### Task B2 — `init()` idempotent lifecycle

```ts
/** Idempotent — runs the check once per app load. Concurrent callers share one in-flight promise. */
async init(): Promise<void> {
  if (this.#initPromise) return this.#initPromise;
  this.#initPromise = this.#runCheckThenMaybeResolve();
  try { await this.#initPromise; } finally { /* keep the promise so re-calls are no-ops */ }
}

async #runCheckThenMaybeResolve(): Promise<void> {
  this.status = 'checking';
  const payload = await this.#transport.check();
  this.apply(payload);
  if (payload.status === 'ok') return;
  await this.#transport.resolve(payload, (ev) => this.applyEvent(ev));
}
```

Tests (TDD red-first):
- `init()` calls `transport.check()` once and applies the result.
- `init()` calls `transport.resolve(...)` iff `check` returns non-ok.
- Concurrent `init()` calls share one promise (transport.check called exactly once).
- Each `HealthEvent` fed back into `applyEvent` mutates state correctly.
- `init()` on an already-ok state DOES NOT call resolve.

### Task B3 — `verify()` (force a fresh check)

```ts
/** Force a fresh check. Clears the session cache. Same idempotency while in flight. */
async verify(): Promise<void> {
  if (typeof sessionStorage !== 'undefined') sessionStorage.removeItem('sensei:health');
  this.#initPromise = null; // allow init() to run again
  return this.init();
}
```

Tests:
- `verify()` clears `sensei:health` from sessionStorage.
- `verify()` causes a fresh `transport.check()` call (separate from any prior `init`).
- `verify()` is also idempotent if called concurrently.

### Task B4 — sessionStorage cache writes from inside `apply()`

When the terminal status flips to `ok`, `apply()` writes `'ready'` to sessionStorage (key `sensei:health`). On any other status, the key is removed. This is the synchronous handle `hooks.client.ts` reads during reroute.

```ts
apply(p: HealthPayload): void {
  // …existing invariant checks…
  // …existing field assignments…
  if (typeof sessionStorage !== 'undefined') {
    if (p.status === 'ok') sessionStorage.setItem('sensei:health', 'ready');
    else                   sessionStorage.removeItem('sensei:health');
  }
}
```

Tests:
- Applying `ok` writes `'ready'`.
- Applying any other status removes the key.
- The test harness stubs `sessionStorage` globally (existing `vi.stubGlobal('sessionStorage', ...)` pattern).

### Task B5 — Remove `appState.healthReady` / `setHealthReady`

The session cache now lives in HealthState. The old `AppState` field and method are dead.

**Files:**
- Modify: `app/src/lib/appstate.svelte.ts` (delete the field + method)
- Modify: `app/src/lib/appstate.spec.svelte.ts` (delete the tests for them)
- Grep & remove all other references (look in `/health/+page.svelte` etc.).

---

## Section C — Page wiring

### Task C1 — `/health/+page.svelte` calls `healthState.init()` on mount

**Files:**
- Modify: `app/src/routes/(health)/health/+page.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { healthState } from '$lib/health-state.svelte.js';
  import HealthView from './HealthView.svelte';

  onMount(() => { healthState.init(); });

  function onEnter()   { goto('/', { replaceState: true }); }
  function onVerify() { healthState.verify(); }
  function onCopyScript() {
    if (healthState.remedy) navigator.clipboard?.writeText(healthState.remedy.script);
  }
</script>

<HealthView state={healthState} {onEnter} {onVerify} {onCopyScript} />
```

The page is still ~14 lines. Just adds the `onMount(init)` and wires `onVerify`.

Tests: existing 6 HealthView.spec tests + 2 new tests:
- Mounting the page calls `healthState.init()` exactly once.
- Clicking the verify button (via the Remedy sub-component) calls `healthState.verify()`.

### Task C2 — `hooks.client.ts` confirms sessionStorage shape

No behavior change — `hooks.client.ts` already reads `sessionStorage.getItem('sensei:health') === 'ready'`. We just verify that B4's write side is wire-compatible with this read side (one targeted test).

---

## Section D — Stop importing health symbols from `bootstrap.ts`

Phase 1c does NOT delete `bootstrap.ts`. It stops _using_ the health-related symbols from it. After Sections B + C, the only callers of `$lib/bootstrap` are those that need the aux wrappers (`detectHardware`, `listModels`, `missingModels`, `getDaemonPort`, `hasTauri`) — those callers stay untouched until the setup-wizard / upgrade-flow phase that comes after this one.

### Task D1 — Audit + remove health-symbol imports

```bash
grep -rn "checkAndFixBootstrap\|listenBootstrapEvents\|listenBootstrapReport\|BootstrapEvent\|BootstrapReport\|GateStatus\|GateReport\|HumanAction\|getPlatform" app/src 2>&1
```

For every remaining caller after Sections B + C, replace the legacy health call with the corresponding `healthState.*` access. Most of these get cleaned up naturally as part of B/C; this task is the audit-and-sweep that guarantees zero stragglers remain.

**Acceptance:** the grep above returns zero hits outside of `app/src/lib/bootstrap.ts` itself. The legacy health exports continue to exist in `bootstrap.ts` as dormant code (deletion is deferred to a later phase, when the aux wrappers also move).

### Task D2 — Rewrite `app/src-tauri/src/commands/update.rs`

The G1 stub for `update.rs` was placeholder. Now we wire it to the new health surface:

- The upgrade flow needs to know: are we currently healthy? Use `bootstrap::check()` directly.
- Reset of `sensei:app-version` in sessionStorage stays the same.
- The "run upgrade steps" path goes through the same `bootstrap::resolve` machinery (brew bundle re-runs, db migrations, etc.) — or invokes a separate `upgrade::run()` function the bootstrap crate exposes. **Design decision deferred** to the start of D2.

---

## Section E — Integration smoke

### Task E1 — Cold-start E2E (manual)

Run the actual Tauri dev build against a wiped environment:

```bash
# Wipe the test env
dropdb sensei_dev 2>/dev/null || true
rm -rf ~/.local/share/sensei  # or wherever sensei stores its files

# Launch
cd app && bun run tauri dev
```

Expected behavior:
1. App boots, navigates to /health.
2. UI shows `checking → resolving` with per-component ledger animation.
3. `brew bundle` runs (or fails with a remedy card if Homebrew is missing).
4. After `brew bundle` succeeds, `database` checks fail (db doesn't exist) → `db_setup` runs → daemon starts.
5. Final state is `ok` → "Enter" button enabled → click → routes to /.
6. SessionStorage `sensei:health = 'ready'` is set.

Failure modes to verify:
- Stop the daemon mid-session → `verify` brings the ledger back to life.
- Delete `sensei_dev` DB → `verify` → re-runs db_setup.

### Task E2 — JSON wire-shape integration test (TS side)

A new test in `app/src/lib/health-transport.spec.svelte.ts` (or its own file) decodes a captured Rust /health response (from `crates/bootstrap/tests/json_wire_shape.rs`) and asserts it parses into `HealthPayload` cleanly via the `RealTransport`. Locks the contract end-to-end.

---

## Verification gate (end of Phase 1c)

- [ ] **Workspace builds:** `cargo build --workspace`
- [ ] **All Rust tests pass:** `cargo test --workspace`
- [ ] **All app tests pass:** `bun run test:unit` — expected ~285+ (275 baseline + 10+ new for transport / lifecycle).
- [ ] **App svelte-check:** 0 errors / 0 warnings.
- [ ] **Coverage gate (TS):** `bun run test:unit -- --coverage` — `health-state.svelte.ts`, `health-transport.ts`, all sub-components, `+page.svelte` all at 100% lines + statements.
- [ ] **Manual E2E smoke:** the cold-start sequence in Task E1.
- [ ] **Push** to `origin/develop` and merge to `main`.

---

## Deferred to a later phase

This list is intentionally long — Phase 1c is the narrow wiring step. Everything else waits.

- **Delete `app/src/lib/bootstrap.ts`** — once the aux wrappers move to `platform-info.ts` / `tauri-env.ts` in the setup-wizard phase, the file becomes deletable.
- **Setup wizard migration** — `detectHardware`, `listModels`, `missingModels`, `getDaemonPort` callers (mostly the wizard) move to their final homes (daemon HTTP endpoints or relocated TS modules). Rust aux commands retire after that.
- **`update.rs` rewrite** — the upgrade flow re-wires on top of the new health surface. The G1 stub stays in place until then.
- **Process-runner injection for Rust resolvers** — closes the 33-77% coverage gap on shell-out resolver code. Refactor `BrewBundleResolver` / `DatabaseResolver` / `DaemonStartResolver` to accept a `dyn CommandRunner`. Same for `PostgresDatabaseChecker`.
- **`latest` version lookup** — the `latest: string | null` field on HealthState that's currently always null. Decide source: GitHub Releases API vs a daemon `/api/version/latest` endpoint.
- **Upgrade redirect** behavior in `hooks.client.ts` when `latest > version`.
- **Scenario registry** (`health-scenarios.ts`) + per-scenario snapshot tests of `HealthView`.

---

## Estimated commit shape

About 9–11 commits, narrower than 1b:

1. `feat(app): add HealthTransport interface + MockTransport` (A1)
2. `feat(app): add RealTransport calling health_check / health_check_and_resolve` (A2)
3. `feat(app): HealthState takes a transport via constructor` (B1)
4. `feat(app): HealthState.init() lifecycle` (B2)
5. `feat(app): HealthState.verify() forces a fresh check` (B3)
6. `feat(app): apply() writes sessionStorage cache for hook reads` (B4)
7. `chore(app): remove appState.healthReady / setHealthReady` (B5)
8. `feat(app): /health page calls init() onMount + wires verify` (C1)
9. `test(app): hooks.client.ts reroute reads HealthState cache` (C2)
10. `refactor(app): sweep remaining legacy bootstrap-health imports` (D1)
11. `chore: phase 1c complete` + push + merge to main.

(D2/D3/D4 from earlier drafts are now deferred — see the "Deferred" section.)

Each commit is reviewable in isolation. The end-of-phase merge is identical to the Phase 1a / 1b pattern.
