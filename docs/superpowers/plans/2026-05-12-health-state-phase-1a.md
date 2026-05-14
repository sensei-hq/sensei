# Health State — Phase 1a Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `HealthState` data class and four UI sub-components (`Header`, `Hero`, `Remedy`, `Ledger`) plus the `HealthView` composition, with 100% TDD-driven coverage. After Phase 1a, the `/health` page renders the cold-load state from the singleton; no transport exists yet.

**Architecture:** A reactive Svelte 5 `$state`-based class lives in `app/src/lib/`, owns the entire `HealthPayload` shape, and exposes `apply()` and `applyEvent()` as the only mutation paths. Four small presentation components in `app/src/routes/(health)/health/` consume POJO props derived from the state. Compile-time discriminated union plus runtime guards in `apply()` enforce the contract that `status === 'needs-action' ⇔ remedy !== null`.

**Tech Stack:** TypeScript, Svelte 5 (runes: `$state`, `$derived`, `$props`), SvelteKit, vitest 4 + jsdom, Svelte 5 `mount()` API for component tests (no `@testing-library/svelte` in the repo).

**Source spec:** `docs/superpowers/specs/2026-05-12-health-state-design.md`

---

## Pre-flight

- [ ] **Confirm working directory and branch**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
pwd
git rev-parse --abbrev-ref HEAD
git status --short
```

Expected: pwd ends in `/sensei`, branch is `develop`, staged/unstaged files limited to the design doc and the writing-plans output. Do not start work until your working tree is on `develop`.

- [ ] **Run the existing test suite to establish a green baseline**

```bash
cd app && bun run test:unit 2>&1 | tail -20
```

Expected: all tests pass. If anything fails, stop — investigate before adding new tests.

- [ ] **Run svelte-check to confirm a green baseline**

```bash
cd app && bun run check 2>&1 | tail -20
```

Expected: 0 errors and 0 warnings, or whatever the current baseline is. Note the number; new code must not increase it.

- [ ] **Install the vitest coverage provider (one-time)**

The repo uses vitest 4 but doesn't ship `@vitest/coverage-v8`. Phase 1a's 100%-coverage gate (Task 18) requires it. Pin to the same major as vitest.

```bash
cd app && bun add -d @vitest/coverage-v8@4
```

Then commit the package.json + lock update:

```bash
git add app/package.json app/bun.lock
git commit -m "chore(app): add @vitest/coverage-v8 for health-state coverage gate"
```

---

## Task 1 — Types and runtime invariant table

**Files:**
- Create: `app/src/lib/health-types.ts`

This task creates the type surface only. There are no tests at this stage — the types are exercised by every subsequent test.

- [ ] **Step 1: Write `health-types.ts`**

```ts
// app/src/lib/health-types.ts

// ── Closed enums ─────────────────────────────────────────────────────────

export type Platform        = 'macos' | 'linux' | 'windows';
export type HealthStatus    = 'checking' | 'resolving' | 'ok' | 'needs-action';
export type ComponentStatus =
  | 'pending' | 'checking' | 'installing' | 'ready' | 'failed';

export type ComponentId      = 'postgres' | 'ollama' | 'sensei' | 'database' | 'daemon';
export type PackageManagerId = 'homebrew' | 'winget';

export const COMPONENT_ORDER: readonly ComponentId[] =
  ['postgres', 'ollama', 'sensei', 'database', 'daemon'] as const;

// ── Component (both ledger rows and package manager use this shape) ──────

export interface Component {
  id: ComponentId | PackageManagerId;
  label: string;
  note: string | null;
  status: ComponentStatus;
  version: string | null;
  detail: string | null;
}

// ── Remedy — opaque strings produced by the Rust crate ───────────────────

export interface Remedy {
  message: string;
  script: string;
  url: string | null;
}

// ── Discriminated union: status === 'needs-action' ⇔ remedy !== null ─────

type PayloadBase = {
  version: string;
  uptimeSeconds: number;
  platform: Platform;
  packageManager: Component;
  components: Component[];
};

export type HealthPayload =
  | (PayloadBase & { status: Exclude<HealthStatus, 'needs-action'>; remedy: null })
  | (PayloadBase & { status: 'needs-action'; remedy: Remedy });

// ── Streaming events ─────────────────────────────────────────────────────

export type HealthEvent =
  | { kind: 'phase';     phase: Extract<HealthStatus, 'checking' | 'resolving'> }
  | { kind: 'component'; id: ComponentId | PackageManagerId; patch: Partial<Component> }
  | { kind: 'remedy';    remedy: Remedy }
  | { kind: 'report';    payload: HealthPayload };
```

- [ ] **Step 2: Verify it type-checks**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: error count unchanged from the pre-flight baseline.

- [ ] **Step 3: Commit**

```bash
git add app/src/lib/health-types.ts
git commit -m "feat(app): add HealthState types and discriminated payload union"
```

---

## Task 2 — HealthState construction and seed application

**Files:**
- Create: `app/src/lib/health-state.svelte.ts`
- Create: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Write the failing test for default construction**

Create `app/src/lib/health-state.spec.svelte.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { HealthState, emptyPayload } from './health-state.svelte.js';
import { COMPONENT_ORDER } from './health-types.js';
import type { HealthPayload, Remedy } from './health-types.js';

const remedyFixture = (): Remedy => ({
  message: 'Run the script in your terminal.',
  script: 'brew bundle --file=https://example/Brewfile',
  url: null,
});

const okPayload = (): HealthPayload => ({
  version: '0.2.14',
  uptimeSeconds: 12,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'ready', version: '4.2.0', detail: null },
  components: COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'ready' as const, version: '1.0.0', detail: null,
  })),
  status: 'ok',
  remedy: null,
});

const needsActionPayload = (): HealthPayload => ({
  ...okPayload(),
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'failed', version: null, detail: 'brew missing' },
  components: COMPONENT_ORDER.map((id) => ({
    id, label: id, note: null, status: 'failed' as const, version: null, detail: 'blocked',
  })),
  status: 'needs-action',
  remedy: remedyFixture(),
});

describe('HealthState — construction', () => {
  it('defaults to the empty payload', () => {
    const s = new HealthState();
    expect(s.status).toBe('checking');
    expect(s.version).toBe('');
    expect(s.platform).toBe('macos');
    expect(s.components).toHaveLength(5);
    expect(s.components.map((c) => c.id)).toEqual([...COMPONENT_ORDER]);
    expect(s.components.every((c) => c.status === 'pending')).toBe(true);
    expect(s.packageManager.id).toBe('homebrew');
    expect(s.remedy).toBeNull();
    expect(s.latest).toBeNull();
  });

  it('applies a seed payload through apply()', () => {
    const s = new HealthState(okPayload());
    expect(s.status).toBe('ok');
    expect(s.version).toBe('0.2.14');
    expect(s.components.every((c) => c.status === 'ready')).toBe(true);
  });

  it('emptyPayload satisfies all invariants (constructor would throw otherwise)', () => {
    expect(() => new HealthState(emptyPayload)).not.toThrow();
  });
});

export { okPayload, needsActionPayload, remedyFixture };
```

- [ ] **Step 2: Run the test, expect RED**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: failures with `Cannot find module './health-state.svelte.js'` or similar — the module doesn't exist yet.

- [ ] **Step 3: Write the minimal implementation**

Create `app/src/lib/health-state.svelte.ts`:

```ts
import type {
  HealthPayload, HealthEvent, Component, HealthStatus, Platform,
  PackageManagerId, ComponentId, Remedy,
} from './health-types.js';
import { COMPONENT_ORDER } from './health-types.js';

function emptyComponent(id: ComponentId): Component {
  return { id, label: id, note: null, status: 'pending', version: null, detail: null };
}

/** Deterministic default — status='checking' so the UI never flashes 'ok' pre-apply. */
export const emptyPayload: HealthPayload = {
  version: '',
  uptimeSeconds: 0,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'pending', version: null, detail: null },
  components: COMPONENT_ORDER.map(emptyComponent),
  status: 'checking',
  remedy: null,
};

export class HealthState {
  status         = $state<HealthStatus>('checking');
  version        = $state<string>('');
  latest         = $state<string | null>(null);
  platform       = $state<Platform>('macos');
  packageManager = $state<Component>(emptyPayload.packageManager);
  components     = $state<Component[]>(emptyPayload.components);
  remedy         = $state<Remedy | null>(null);

  constructor(seed: HealthPayload = emptyPayload) {
    this.apply(seed);
  }

  apply(p: HealthPayload): void {
    this.version        = p.version;
    this.platform       = p.platform;
    this.packageManager = p.packageManager;
    this.components     = p.components;
    this.remedy         = p.remedy;
    this.status         = p.status;
  }
}

export const healthState = new HealthState();
```

- [ ] **Step 4: Run the test, expect GREEN**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: 3 tests pass.

- [ ] **Step 5: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: error count unchanged from baseline.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/health-state.svelte.ts app/src/lib/health-state.spec.svelte.ts
git commit -m "feat(app): scaffold HealthState class with construction tests"
```

---

## Task 3 — `apply()` happy paths for all four statuses

**Files:**
- Modify: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Add tests for each status**

Append to the existing `health-state.spec.svelte.ts`:

```ts
describe('HealthState — apply() happy paths', () => {
  it('applies an ok payload', () => {
    const s = new HealthState();
    s.apply(okPayload());
    expect(s.status).toBe('ok');
    expect(s.remedy).toBeNull();
    expect(s.components.every((c) => c.status === 'ready')).toBe(true);
  });

  it('applies a needs-action payload (remedy is set)', () => {
    const s = new HealthState();
    s.apply(needsActionPayload());
    expect(s.status).toBe('needs-action');
    expect(s.remedy?.script).toContain('brew bundle');
  });

  it('applies a resolving payload (remedy cleared)', () => {
    const s = new HealthState(needsActionPayload());
    s.apply({ ...okPayload(), status: 'resolving', remedy: null });
    expect(s.status).toBe('resolving');
    expect(s.remedy).toBeNull();
  });

  it('applies a checking payload', () => {
    const s = new HealthState(okPayload());
    s.apply({ ...okPayload(), status: 'checking', remedy: null });
    expect(s.status).toBe('checking');
  });

  it('replaces fields rather than merging on successive apply()', () => {
    const s = new HealthState(okPayload());
    s.apply({ ...okPayload(), version: '9.9.9', uptimeSeconds: 999 });
    expect(s.version).toBe('9.9.9');
  });
});
```

- [ ] **Step 2: Run, expect GREEN (apply already implemented)**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: all new tests pass — `apply()` already handles these.

- [ ] **Step 3: Commit**

```bash
git add app/src/lib/health-state.spec.svelte.ts
git commit -m "test(app): cover HealthState.apply() happy paths"
```

---

## Task 4 — `apply()` runtime invariants (INV-1, INV-2, INV-3)

**Files:**
- Modify: `app/src/lib/health-state.svelte.ts`
- Modify: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Write failing tests for each invariant**

Append to `health-state.spec.svelte.ts`:

```ts
describe('HealthState — apply() invariants', () => {
  it('INV-1: needs-action with null remedy throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), status: 'needs-action', remedy: null } as unknown as HealthPayload;
    expect(() => s.apply(bad)).toThrow(/needs-action requires a remedy/);
  });

  it('INV-1: non-needs-action with non-null remedy throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), status: 'ok', remedy: remedyFixture() } as unknown as HealthPayload;
    expect(() => s.apply(bad)).toThrow(/must not carry a remedy/);
  });

  it('INV-2: wrong components length throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), components: okPayload().components.slice(0, 4) };
    expect(() => s.apply(bad)).toThrow(/expected 5 components, got 4/);
  });

  it('INV-2: wrong components order throws', () => {
    const s = new HealthState();
    const reordered = okPayload();
    [reordered.components[0], reordered.components[1]] = [reordered.components[1], reordered.components[0]];
    expect(() => s.apply(reordered)).toThrow(/components\[0\]\.id must be "postgres"/);
  });

  it('INV-3: macos platform with winget package manager throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), platform: 'macos' as const,
      packageManager: { ...okPayload().packageManager, id: 'winget' as const } };
    expect(() => s.apply(bad)).toThrow(/platform=macos expects packageManager.id="homebrew"/);
  });

  it('INV-3: windows platform with homebrew package manager throws', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), platform: 'windows' as const,
      packageManager: { ...okPayload().packageManager, id: 'homebrew' as const } };
    expect(() => s.apply(bad)).toThrow(/platform=windows expects packageManager.id="winget"/);
  });

  it('INV-3: linux platform requires homebrew', () => {
    const s = new HealthState();
    const bad = { ...okPayload(), platform: 'linux' as const,
      packageManager: { ...okPayload().packageManager, id: 'winget' as const } };
    expect(() => s.apply(bad)).toThrow(/platform=linux expects packageManager.id="homebrew"/);
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: 7 new tests fail — no throws yet. Construction tests still pass.

- [ ] **Step 3: Add the runtime guards to `apply()`**

Replace the `apply()` method in `app/src/lib/health-state.svelte.ts`:

```ts
  apply(p: HealthPayload): void {
    // INV-1: discriminated union covers compile-time; this is the runtime boundary
    if (p.status === 'needs-action' && p.remedy === null)
      throw new Error('HealthState: needs-action requires a remedy');
    if (p.status !== 'needs-action' && p.remedy !== null)
      throw new Error(`HealthState: status=${p.status} must not carry a remedy`);

    // INV-2
    if (p.components.length !== COMPONENT_ORDER.length)
      throw new Error(`HealthState: expected ${COMPONENT_ORDER.length} components, got ${p.components.length}`);
    for (let i = 0; i < COMPONENT_ORDER.length; i++) {
      if (p.components[i].id !== COMPONENT_ORDER[i])
        throw new Error(`HealthState: components[${i}].id must be "${COMPONENT_ORDER[i]}", got "${p.components[i].id}"`);
    }

    // INV-3
    const expectedPm: PackageManagerId = p.platform === 'windows' ? 'winget' : 'homebrew';
    if (p.packageManager.id !== expectedPm)
      throw new Error(`HealthState: platform=${p.platform} expects packageManager.id="${expectedPm}", got "${p.packageManager.id}"`);

    this.version        = p.version;
    this.platform       = p.platform;
    this.packageManager = p.packageManager;
    this.components     = p.components;
    this.remedy         = p.remedy;
    this.status         = p.status;
  }
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: error count unchanged.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/health-state.svelte.ts app/src/lib/health-state.spec.svelte.ts
git commit -m "feat(app): enforce HealthState apply() invariants INV-1/2/3"
```

---

## Task 5 — `applyEvent('phase')`

**Files:**
- Modify: `app/src/lib/health-state.svelte.ts`
- Modify: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Write the failing test**

Append:

```ts
describe('HealthState — applyEvent("phase")', () => {
  it('sets status to checking', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'phase', phase: 'checking' });
    expect(s.status).toBe('checking');
  });

  it('sets status to resolving', () => {
    const s = new HealthState();
    s.applyEvent({ kind: 'phase', phase: 'resolving' });
    expect(s.status).toBe('resolving');
  });
});
```

- [ ] **Step 2: Run, expect RED (no `applyEvent` method yet)**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: `s.applyEvent is not a function`.

- [ ] **Step 3: Add `applyEvent` with phase handling**

Add this method to `HealthState` (after `apply`):

```ts
  applyEvent(e: HealthEvent): void {
    switch (e.kind) {
      case 'phase':     this.status = e.phase; return;
      case 'component': return; // implemented in Task 6
      case 'remedy':    return; // implemented in Task 7
      case 'report':    return; // implemented in Task 8
      default: {
        const _exhaustive: never = e;
        throw new Error(`HealthState: unknown event kind ${JSON.stringify(_exhaustive)}`);
      }
    }
  }
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: phase tests pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/health-state.svelte.ts app/src/lib/health-state.spec.svelte.ts
git commit -m "feat(app): HealthState.applyEvent handles phase events"
```

---

## Task 6 — `applyEvent('component')` and INV-4

**Files:**
- Modify: `app/src/lib/health-state.svelte.ts`
- Modify: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

Append:

```ts
describe('HealthState — applyEvent("component")', () => {
  it('patches a known ledger component, leaves others intact', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'component', id: 'postgres', patch: { status: 'installing' } });
    expect(s.components[0].status).toBe('installing');
    expect(s.components[1].status).toBe('ready');
    expect(s.components[2].status).toBe('ready');
  });

  it('patches the package manager', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'component', id: 'homebrew', patch: { detail: 'permission denied' } });
    expect(s.packageManager.detail).toBe('permission denied');
    expect(s.packageManager.status).toBe('ready'); // un-patched fields intact
  });

  it('patches multiple fields at once', () => {
    const s = new HealthState(okPayload());
    s.applyEvent({ kind: 'component', id: 'daemon',
      patch: { status: 'failed', detail: 'port in use' } });
    expect(s.components[4].status).toBe('failed');
    expect(s.components[4].detail).toBe('port in use');
  });

  it('INV-4: unknown component id throws', () => {
    const s = new HealthState();
    expect(() =>
      s.applyEvent({ kind: 'component', id: 'not-a-thing' as never, patch: {} })
    ).toThrow(/unknown component id "not-a-thing"/);
  });
});
```

- [ ] **Step 2: Run, expect RED on the patch tests**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: all 4 component tests fail — the case in `applyEvent` is a no-op.

- [ ] **Step 3: Implement `#patch` and wire the case**

Add the private method to `HealthState`:

```ts
  #patch(id: ComponentId | PackageManagerId, patch: Partial<Component>): void {
    if (id === this.packageManager.id) {
      this.packageManager = { ...this.packageManager, ...patch };
      return;
    }
    const idx = this.components.findIndex((c) => c.id === id);
    if (idx < 0) {
      throw new Error(
        `HealthState: unknown component id "${id}" — not in [${this.packageManager.id}, ${COMPONENT_ORDER.join(', ')}]`
      );
    }
    const next = this.components.slice();
    next[idx] = { ...next[idx], ...patch };
    this.components = next;
  }
```

Replace the `case 'component':` branch in `applyEvent` with:

```ts
      case 'component': this.#patch(e.id, e.patch); return;
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/health-state.svelte.ts app/src/lib/health-state.spec.svelte.ts
git commit -m "feat(app): HealthState.applyEvent component patch + INV-4 throw"
```

---

## Task 7 — `applyEvent('remedy')`

**Files:**
- Modify: `app/src/lib/health-state.svelte.ts`
- Modify: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Write the failing test**

```ts
describe('HealthState — applyEvent("remedy")', () => {
  it('replaces remedy regardless of previous value', () => {
    const s = new HealthState();
    const r1 = remedyFixture();
    s.applyEvent({ kind: 'remedy', remedy: r1 });
    expect(s.remedy).toEqual(r1);
    const r2 = { ...r1, message: 'new message' };
    s.applyEvent({ kind: 'remedy', remedy: r2 });
    expect(s.remedy?.message).toBe('new message');
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: remedy not updated.

- [ ] **Step 3: Implement**

Replace the `case 'remedy':` line:

```ts
      case 'remedy': this.remedy = e.remedy; return;
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/health-state.svelte.ts app/src/lib/health-state.spec.svelte.ts
git commit -m "feat(app): HealthState.applyEvent remedy"
```

---

## Task 8 — `applyEvent('report')` and INV-5 exhaustive throw

**Files:**
- Modify: `app/src/lib/health-state.svelte.ts`
- Modify: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

```ts
describe('HealthState — applyEvent("report")', () => {
  it('terminal report is equivalent to apply()', () => {
    const s = new HealthState();
    s.applyEvent({ kind: 'report', payload: needsActionPayload() });
    expect(s.status).toBe('needs-action');
    expect(s.remedy).not.toBeNull();
    expect(s.components.every((c) => c.status === 'failed')).toBe(true);
  });
});

describe('HealthState — applyEvent INV-5', () => {
  it('unknown event kind throws', () => {
    const s = new HealthState();
    expect(() => s.applyEvent({ kind: 'bogus' } as never))
      .toThrow(/unknown event kind/);
  });
});
```

- [ ] **Step 2: Run, expect RED on the report test**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: report does nothing yet; the unknown-kind test already passes thanks to the default branch.

- [ ] **Step 3: Implement the `report` branch**

Replace the `case 'report':` line:

```ts
      case 'report': this.apply(e.payload); return;
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/health-state.svelte.ts app/src/lib/health-state.spec.svelte.ts
git commit -m "feat(app): HealthState.applyEvent report path + INV-5 throw"
```

---

## Task 9 — Derived getters (`isOk`, `isBusy`, `needsAction`)

**Files:**
- Modify: `app/src/lib/health-state.svelte.ts`
- Modify: `app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

```ts
describe('HealthState — derived getters', () => {
  it.each([
    ['checking',     { isOk: false, isBusy: true,  needsAction: false }],
    ['resolving',    { isOk: false, isBusy: true,  needsAction: false }],
    ['ok',           { isOk: true,  isBusy: false, needsAction: false }],
    ['needs-action', { isOk: false, isBusy: false, needsAction: true  }],
  ] as const)('status=%s → isOk/isBusy/needsAction', (status, expected) => {
    const s = new HealthState();
    if (status === 'needs-action') {
      s.apply(needsActionPayload());
    } else {
      s.apply({ ...okPayload(), status, remedy: null });
    }
    expect(s.isOk).toBe(expected.isOk);
    expect(s.isBusy).toBe(expected.isBusy);
    expect(s.needsAction).toBe(expected.needsAction);
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: undefined getters.

- [ ] **Step 3: Add the getters**

Insert after the field declarations and before `constructor` in `HealthState`:

```ts
  get isOk():        boolean { return this.status === 'ok'; }
  get isBusy():      boolean { return this.status === 'checking' || this.status === 'resolving'; }
  get needsAction(): boolean { return this.status === 'needs-action'; }
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts 2>&1 | tail -20
```

Expected: all 4 parametrised cases pass.

- [ ] **Step 5: svelte-check + coverage spot check**

```bash
cd app && bun run check 2>&1 | tail -10
cd app && bun run test:unit -- src/lib/health-state.spec.svelte.ts --coverage 2>&1 | tail -25
```

Expected: 0 new check errors. Coverage on `health-state.svelte.ts` reaches 100% line + branch.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/health-state.svelte.ts app/src/lib/health-state.spec.svelte.ts
git commit -m "feat(app): HealthState derived getters (isOk, isBusy, needsAction)"
```

---

## Task 10 — Component test harness helper

**Files:**
- Create: `app/src/lib/test-mount.ts`

This task creates a tiny helper so component tests stay readable. The repo has no `@testing-library/svelte`, so we use Svelte 5's built-in `mount` / `unmount` API.

- [ ] **Step 1: Write the helper**

```ts
// app/src/lib/test-mount.ts
import { mount, unmount } from 'svelte';
import type { Component } from 'svelte';

export interface MountResult {
  container: HTMLElement;
  destroy: () => void;
}

/**
 * Mount a Svelte 5 component into a fresh detached container appended to <body>.
 * Reactive tests should mutate a reactive object (e.g. a `HealthState` instance)
 * passed as a prop — Svelte 5's fine-grained reactivity propagates automatically.
 * Tests must call `destroy()` (typically in afterEach).
 */
export function mountComponent<P extends Record<string, unknown>>(
  comp: Component<P>,
  props: P,
): MountResult {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(comp, { target: container, props });
  return {
    container,
    destroy: () => { unmount(instance); container.remove(); },
  };
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: 0 new errors.

- [ ] **Step 3: Commit**

```bash
git add app/src/lib/test-mount.ts
git commit -m "test(app): add mountComponent helper for Svelte 5 component tests"
```

---

## Task 11 — `Header.svelte` + tests

**Files:**
- Create: `app/src/routes/(health)/health/Header.svelte`
- Create: `app/src/routes/(health)/health/Header.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

```ts
// app/src/routes/(health)/health/Header.spec.svelte.ts
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Header from './Header.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Header', () => {
  it.each([
    ['macos',   'macOS'],
    ['linux',   'Linux'],
    ['windows', 'Windows'],
  ] as const)('renders platform label for %s', (platform, label) => {
    const m = mountComponent(Header, { platform, status: 'checking' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain(label);
  });

  it('renders the "ok" headline', () => {
    const m = mountComponent(Header, { platform: 'macos', status: 'ok' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('The foundation');
    expect(m.container.textContent).toContain('holds');
  });

  it('renders the "resolving" headline', () => {
    const m = mountComponent(Header, { platform: 'macos', status: 'resolving' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Setting up your.*foundation/);
  });

  it('renders the "needs-action" headline', () => {
    const m = mountComponent(Header, { platform: 'macos', status: 'needs-action' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('One last');
  });

  it('renders the "checking" headline', () => {
    const m = mountComponent(Header, { platform: 'macos', status: 'checking' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Checking the.*foundation/i);
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Header.spec.svelte.ts 2>&1 | tail -20
```

Expected: module not found.

- [ ] **Step 3: Write the component**

```svelte
<!-- app/src/routes/(health)/health/Header.svelte -->
<script lang="ts">
  import type { Platform, HealthStatus } from '$lib/health-types.js';

  interface Props { platform: Platform; status: HealthStatus; }
  let { platform, status }: Props = $props();

  const PLATFORM_LABEL: Record<Platform, string> = {
    macos: 'macOS', linux: 'Linux', windows: 'Windows',
  };
</script>

<header class="flex flex-col gap-3.5 mb-9">
  <div class="flex items-center gap-2.5">
    <span class="kanji text-xl text-primary-z5">支</span>
    <span class="text-2xs tracking-tag uppercase text-surface-z6">
      bootstrap · {PLATFORM_LABEL[platform]}
    </span>
  </div>

  <h1 class="display text-4xl font-light leading-tight tracking-tight">
    {#if status === 'ok'}
      The foundation <span class="text-success-z5">holds.</span>
    {:else if status === 'resolving'}
      Setting up your <span class="text-primary-z5">foundation.</span>
    {:else if status === 'checking'}
      Checking the <span class="text-surface-z7">foundation…</span>
    {:else}
      One last <span class="text-primary-z5">step.</span>
    {/if}
  </h1>
</header>
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Header.spec.svelte.ts 2>&1 | tail -20
```

Expected: all 7 tests pass.

- [ ] **Step 5: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: 0 new errors.

- [ ] **Step 6: Commit**

```bash
git add 'app/src/routes/(health)/health/Header.svelte' 'app/src/routes/(health)/health/Header.spec.svelte.ts'
git commit -m "feat(app): add Header sub-component with platform + status tests"
```

---

## Task 12 — `Hero.svelte` + tests

**Files:**
- Create: `app/src/routes/(health)/health/Hero.svelte`
- Create: `app/src/routes/(health)/health/Hero.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

```ts
// app/src/routes/(health)/health/Hero.spec.svelte.ts
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Hero from './Hero.svelte';
import type { Component } from '$lib/health-types.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const pm = (overrides: Partial<Component> = {}): Component => ({
  id: 'homebrew', label: 'Homebrew', note: null,
  status: 'ready', version: '4.2.0', detail: null, ...overrides,
});

const allReady = (): Component[] => COMPONENT_ORDER.map((id) => ({
  id, label: id, note: null, status: 'ready' as const, version: '1.0', detail: null,
}));

const oneInstalling = (idx: number): Component[] => {
  const cs = allReady();
  cs[idx] = { ...cs[idx], status: 'installing' };
  return cs;
};

describe('Hero', () => {
  it('renders the package manager label', () => {
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'ok', components: allReady(),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Homebrew');
  });

  it('shows "Enter" button only when status is ok', () => {
    const m1 = mountComponent(Hero, {
      packageManager: pm(), status: 'ok', components: allReady(),
    });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('button[data-action="enter"]')).not.toBeNull();

    const m2 = mountComponent(Hero, {
      packageManager: pm(), status: 'checking', components: allReady(),
    });
    cleanup.push(m2.destroy);
    expect(m2.container.querySelector('button[data-action="enter"]')).toBeNull();
  });

  it('Enter button calls onEnter', () => {
    const onEnter = vi.fn();
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'ok', components: allReady(), onEnter,
    });
    cleanup.push(m.destroy);
    const btn = m.container.querySelector('button[data-action="enter"]') as HTMLButtonElement;
    btn.click();
    expect(onEnter).toHaveBeenCalledTimes(1);
  });

  it('shows "Detected. All dependencies installed." copy when status=ok', () => {
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'ok', components: allReady(),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Detected');
  });

  it('shows installing copy with the active component label when status=resolving', () => {
    const cs = oneInstalling(2); // sensei
    cs[2].label = 'Sensei components';
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'resolving', components: cs,
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Installing');
    expect(m.container.textContent).toContain('Sensei components');
    expect(m.container.textContent).toContain('(3/5)');
  });

  it('falls back to ready-count progress when nothing is installing', () => {
    const cs = allReady();
    cs[3].status = 'pending'; cs[4].status = 'pending';
    const m = mountComponent(Hero, {
      packageManager: pm(), status: 'resolving', components: cs,
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/3\/5|\(3\/5\)/);
  });

  it('shows manual copy when status=needs-action', () => {
    const m = mountComponent(Hero, {
      packageManager: pm({ status: 'failed' }), status: 'needs-action', components: allReady(),
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Couldn['’]t finish/);
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Hero.spec.svelte.ts 2>&1 | tail -25
```

Expected: module not found.

- [ ] **Step 3: Write the component**

```svelte
<!-- app/src/routes/(health)/health/Hero.svelte -->
<script lang="ts">
  import type { Component, HealthStatus } from '$lib/health-types.js';

  interface Props {
    packageManager: Component;
    status: HealthStatus;
    components: Component[];
    onEnter?: () => void;
  }
  let { packageManager, status, components, onEnter }: Props = $props();

  const total = components.length;
  const activeIdx = $derived.by(() => {
    const i = components.findIndex(
      (c) => c.status === 'installing' || c.status === 'checking'
    );
    if (i >= 0) return i;
    return components.filter((c) => c.status === 'ready').length;
  });
  const activeLabel = $derived(
    components[Math.min(activeIdx, total - 1)]?.label ?? ''
  );
  const activeCount = $derived(Math.min(activeIdx + 1, total));
</script>

<section class="border border-surface-z2 rounded-xl bg-surface-z2 p-6.5">
  <div class="flex items-center gap-4.5">
    <div class="w-14 h-14 rounded-full border-[1.5px] flex items-center justify-center shrink-0"
         class:border-success-z5={status === 'ok'}
         class:border-primary-z5={status === 'needs-action'}
         class:border-surface-z5={status === 'checking' || status === 'resolving'}>
      {#if status === 'ok'}
        <span class="text-2xl text-success-z5 leading-none">✓</span>
      {:else if status === 'needs-action'}
        <span class="kanji text-xl text-primary-z5">?</span>
      {:else}
        <span class="spinner-ring"></span>
      {/if}
    </div>

    <div class="flex-1 min-w-0">
      <div class="flex items-baseline gap-2.5 mb-1">
        <span class="display text-prose font-medium">{packageManager.label}</span>
        {#if packageManager.note}
          <span class="mono text-2xs text-surface-z5">{packageManager.note}</span>
        {/if}
      </div>
      <div class="text-sm text-surface-z7 leading-snug">
        {#if status === 'ok'}
          Detected. All dependencies installed.
        {:else if status === 'needs-action'}
          Couldn’t finish automatically. Run the script below.
        {:else if status === 'resolving'}
          Detected. Installing
          <span class="text-surface-z9">{activeLabel}</span>
          <span class="mono text-2xs text-surface-z5 ml-2">({activeCount}/{total})</span>
        {:else}
          Checking system…
        {/if}
      </div>
    </div>

    {#if status === 'ok'}
      <button data-action="enter" class="btn-solid shrink-0" onclick={onEnter}>Enter</button>
    {/if}
  </div>
</section>

<style>
  .spinner-ring {
    display: block;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid oklch(var(--color-surface-z5) / 1);
    border-top-color: transparent;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Hero.spec.svelte.ts 2>&1 | tail -20
```

Expected: all 7 tests pass.

- [ ] **Step 5: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: 0 new errors.

- [ ] **Step 6: Commit**

```bash
git add 'app/src/routes/(health)/health/Hero.svelte' 'app/src/routes/(health)/health/Hero.spec.svelte.ts'
git commit -m "feat(app): add Hero sub-component with active-item derivation"
```

---

## Task 13 — `Remedy.svelte` + tests

**Files:**
- Create: `app/src/routes/(health)/health/Remedy.svelte`
- Create: `app/src/routes/(health)/health/Remedy.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

```ts
// app/src/routes/(health)/health/Remedy.spec.svelte.ts
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Remedy from './Remedy.svelte';
import type { Remedy as RemedyT } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const fixture = (over: Partial<RemedyT> = {}): RemedyT => ({
  message: 'Run the script in your terminal.',
  script: 'brew bundle --file=https://example/Brewfile',
  url: null, ...over,
});

describe('Remedy', () => {
  it('renders message and script verbatim', () => {
    const r = fixture();
    const m = mountComponent(Remedy, { remedy: r });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain(r.message);
    const pre = m.container.querySelector('pre');
    expect(pre?.textContent).toBe(r.script);
  });

  it('Copy button calls onCopyScript', () => {
    const onCopyScript = vi.fn();
    const m = mountComponent(Remedy, { remedy: fixture(), onCopyScript });
    cleanup.push(m.destroy);
    (m.container.querySelector('button[data-action="copy"]') as HTMLButtonElement).click();
    expect(onCopyScript).toHaveBeenCalledTimes(1);
  });

  it('Recheck button calls onRecheck', () => {
    const onRecheck = vi.fn();
    const m = mountComponent(Remedy, { remedy: fixture(), onRecheck });
    cleanup.push(m.destroy);
    (m.container.querySelector('button[data-action="recheck"]') as HTMLButtonElement).click();
    expect(onRecheck).toHaveBeenCalledTimes(1);
  });

  it('renders a link only when remedy.url is non-null', () => {
    const m1 = mountComponent(Remedy, { remedy: fixture({ url: null }) });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('a[data-role="remedy-url"]')).toBeNull();

    const m2 = mountComponent(Remedy, {
      remedy: fixture({ url: 'https://brew.sh' }),
    });
    cleanup.push(m2.destroy);
    const a = m2.container.querySelector('a[data-role="remedy-url"]') as HTMLAnchorElement;
    expect(a?.href).toContain('brew.sh');
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Remedy.spec.svelte.ts 2>&1 | tail -20
```

- [ ] **Step 3: Write the component**

```svelte
<!-- app/src/routes/(health)/health/Remedy.svelte -->
<script lang="ts">
  import type { Remedy } from '$lib/health-types.js';

  interface Props {
    remedy: Remedy;
    onCopyScript?: () => void;
    onRecheck?: () => void;
  }
  let { remedy, onCopyScript, onRecheck }: Props = $props();
</script>

<section class="mt-4.5 border border-primary-z5/30 rounded-xl bg-surface-z1 overflow-hidden">
  <header class="flex items-center gap-2.5 px-4.5 py-3.5 border-b border-surface-z2">
    <span class="kanji text-base text-primary-z5">手</span>
    <div class="flex-1">
      <div class="text-sm text-surface-z9">Run this in your terminal</div>
      <div class="text-2xs text-surface-z5 mt-0.5">{remedy.message}</div>
    </div>
    {#if remedy.url}
      <a data-role="remedy-url" href={remedy.url} target="_blank" rel="noopener noreferrer"
         class="text-2xs text-surface-z6 underline">Learn more</a>
    {/if}
  </header>

  <pre class="m-0 px-4.5 py-4 mono text-xs text-surface-z9 bg-surface-z1 leading-relaxed whitespace-pre-wrap break-words max-h-56 overflow-auto">{remedy.script}</pre>

  <footer class="flex items-center justify-between gap-2.5 px-4.5 py-3 border-t border-surface-z2">
    <button data-action="copy" class="btn-solid btn-sm" onclick={onCopyScript}>Copy script</button>
    <button data-action="recheck"
            class="btn-outline btn-sm"
            style="color: oklch(var(--color-primary-z5) / 1); border-color: oklch(var(--color-primary-z5) / 0.4);"
            onclick={onRecheck}>
      I’ve run it · re-check
    </button>
  </footer>
</section>
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Remedy.spec.svelte.ts 2>&1 | tail -20
```

- [ ] **Step 5: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add 'app/src/routes/(health)/health/Remedy.svelte' 'app/src/routes/(health)/health/Remedy.spec.svelte.ts'
git commit -m "feat(app): add Remedy sub-component with copy + recheck callbacks"
```

---

## Task 14 — `Ledger.svelte` + tests

**Files:**
- Create: `app/src/routes/(health)/health/Ledger.svelte`
- Create: `app/src/routes/(health)/health/Ledger.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

```ts
// app/src/routes/(health)/health/Ledger.spec.svelte.ts
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Ledger from './Ledger.svelte';
import type { Component, ComponentStatus } from '$lib/health-types.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const row = (id: Component['id'], status: ComponentStatus, detail: string | null = null): Component => ({
  id, label: String(id), note: null, status, version: null, detail,
});

const allReady = (): Component[] => COMPONENT_ORDER.map((id) => row(id, 'ready'));

describe('Ledger', () => {
  it('renders 5 rows in COMPONENT_ORDER order', () => {
    const m = mountComponent(Ledger, { components: allReady() });
    cleanup.push(m.destroy);
    const labels = Array.from(m.container.querySelectorAll('[data-row]'))
      .map((el) => el.getAttribute('data-row'));
    expect(labels).toEqual([...COMPONENT_ORDER]);
  });

  it.each(
    (['pending', 'checking', 'installing', 'ready', 'failed'] as ComponentStatus[]).map(
      (s) => [s] as const,
    ),
  )('renders the %s badge', (s) => {
    const cs = allReady();
    cs[0] = row('postgres', s);
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const badge = m.container.querySelector('[data-row="postgres"] [data-badge]');
    expect(badge?.textContent?.trim().toLowerCase()).toBe(s);
  });

  it('failed row shows detail text', () => {
    const cs = allReady();
    cs[1] = row('ollama', 'failed', 'port 11434 in use');
    const m = mountComponent(Ledger, { components: cs });
    cleanup.push(m.destroy);
    const detail = m.container.querySelector('[data-row="ollama"] [data-detail]');
    expect(detail?.textContent).toContain('port 11434 in use');
  });

  it('throws when components.length is not 5', () => {
    expect(() =>
      mountComponent(Ledger, { components: allReady().slice(0, 4) })
    ).toThrow(/expected 5 components/);
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Ledger.spec.svelte.ts 2>&1 | tail -20
```

- [ ] **Step 3: Write the component**

```svelte
<!-- app/src/routes/(health)/health/Ledger.svelte -->
<script lang="ts">
  import type { Component } from '$lib/health-types.js';
  import { COMPONENT_ORDER } from '$lib/health-types.js';

  interface Props { components: Component[]; }
  let { components }: Props = $props();

  if (components.length !== COMPONENT_ORDER.length) {
    throw new Error(`Ledger: expected 5 components, got ${components.length}`);
  }

  const dotClass = (s: Component['status']): string => {
    switch (s) {
      case 'ready':      return 'bg-success-z5';
      case 'installing':
      case 'checking':   return 'bg-primary-z5';
      case 'failed':     return 'bg-primary-z5/50';
      case 'pending':    return 'bg-surface-z4';
    }
  };

  const badgeClass = (s: Component['status']): string => {
    switch (s) {
      case 'ready':      return 'text-success-z5';
      case 'installing':
      case 'checking':   return 'text-primary-z5';
      case 'failed':     return 'text-primary-z5/70';
      case 'pending':    return 'text-surface-z5';
    }
  };
</script>

<section class="mt-5.5">
  <div class="text-2xs tracking-tag uppercase text-surface-z5 mb-2.5">what this resolves</div>
  <ul class="flex flex-col">
    {#each components as c (c.id)}
      <li data-row={c.id}
          class="grid grid-cols-[10px_1fr_auto] gap-3 items-center py-2 border-b border-surface-z2"
          style="opacity: {c.status === 'pending' ? 0.55 : 1}">
        <span class="w-2 h-2 rounded-full shrink-0 {dotClass(c.status)}"></span>
        <div>
          <span class="text-sm text-surface-z9">{c.label}</span>
          {#if c.note}<span class="text-xs text-surface-z5 ml-2">· {c.note}</span>{/if}
          {#if c.status === 'failed' && c.detail}
            <div data-detail class="text-2xs text-surface-z6 mt-0.5">{c.detail}</div>
          {/if}
        </div>
        <span data-badge class="mono text-2xs tracking-wider uppercase {badgeClass(c.status)}">
          {c.status}
        </span>
      </li>
    {/each}
  </ul>
</section>
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/Ledger.spec.svelte.ts 2>&1 | tail -25
```

Expected: 5 status tests + structure tests + failed-detail + throw on bad length = 8 tests pass.

- [ ] **Step 5: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add 'app/src/routes/(health)/health/Ledger.svelte' 'app/src/routes/(health)/health/Ledger.spec.svelte.ts'
git commit -m "feat(app): add Ledger sub-component with per-status badge map"
```

---

## Task 15 — `HealthView.svelte` composition + tests

**Files:**
- Create: `app/src/routes/(health)/health/HealthView.svelte`
- Create: `app/src/routes/(health)/health/HealthView.spec.svelte.ts`

- [ ] **Step 1: Write the failing tests**

```ts
// app/src/routes/(health)/health/HealthView.spec.svelte.ts
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import HealthView from './HealthView.svelte';
import { HealthState } from '$lib/health-state.svelte.js';
import { COMPONENT_ORDER } from '$lib/health-types.js';
import type { HealthPayload, Remedy } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const remedy = (): Remedy => ({ message: 'msg', script: 'cmd', url: null });

const ok = (): HealthPayload => ({
  version: '0.2.14', uptimeSeconds: 0, platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'ready', version: '4.2.0', detail: null },
  components: COMPONENT_ORDER.map((id) => ({ id, label: String(id), note: null, status: 'ready' as const, version: '1.0', detail: null })),
  status: 'ok', remedy: null,
});

const needsAction = (): HealthPayload => ({
  ...ok(),
  packageManager: { ...ok().packageManager, status: 'failed' },
  components: COMPONENT_ORDER.map((id) => ({ id, label: String(id), note: null, status: 'failed' as const, version: null, detail: 'blocked' })),
  status: 'needs-action', remedy: remedy(),
});

describe('HealthView', () => {
  it('mounts all four sub-components', () => {
    const state = new HealthState(ok());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('header')).not.toBeNull();          // Header
    expect(m.container.querySelector('section')).not.toBeNull();         // Hero is first <section>
    expect(m.container.querySelector('ul')).not.toBeNull();              // Ledger
  });

  it('does NOT render Remedy when status is not needs-action', () => {
    const state = new HealthState(ok());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('pre')).toBeNull();
  });

  it('renders Remedy when status is needs-action', () => {
    const state = new HealthState(needsAction());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('pre')).not.toBeNull();
  });

  it('renders "Continue →" footer button iff state.isOk', () => {
    const okState = new HealthState(ok());
    const m1 = mountComponent(HealthView, { state: okState });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('button[data-action="continue"]')).not.toBeNull();

    const naState = new HealthState(needsAction());
    const m2 = mountComponent(HealthView, { state: naState });
    cleanup.push(m2.destroy);
    expect(m2.container.querySelector('button[data-action="continue"]')).toBeNull();
  });

  it('Continue button calls onEnter', () => {
    const onEnter = vi.fn();
    const state = new HealthState(ok());
    const m = mountComponent(HealthView, { state, onEnter });
    cleanup.push(m.destroy);
    (m.container.querySelector('button[data-action="continue"]') as HTMLButtonElement).click();
    expect(onEnter).toHaveBeenCalledTimes(1);
  });

  it('reactively toggles Remedy + Continue when state.status flips', async () => {
    const state = new HealthState(needsAction());
    const m = mountComponent(HealthView, { state });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('pre')).not.toBeNull();
    expect(m.container.querySelector('button[data-action="continue"]')).toBeNull();

    state.apply(ok());
    await tick();
    expect(m.container.querySelector('pre')).toBeNull();
    expect(m.container.querySelector('button[data-action="continue"]')).not.toBeNull();
  });
});
```

- [ ] **Step 2: Run, expect RED**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/HealthView.spec.svelte.ts 2>&1 | tail -20
```

- [ ] **Step 3: Write the component**

```svelte
<!-- app/src/routes/(health)/health/HealthView.svelte -->
<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import Header from './Header.svelte';
  import Hero   from './Hero.svelte';
  import Remedy from './Remedy.svelte';
  import Ledger from './Ledger.svelte';

  interface Props {
    state: HealthState;
    onEnter?: () => void;
    onRecheck?: () => void;
    onCopyScript?: () => void;
  }
  let { state, onEnter, onRecheck, onCopyScript }: Props = $props();
</script>

<div class="flex-1 min-h-0 overflow-y-auto px-10 py-12">
  <div class="max-w-[640px] w-full mx-auto">
    <Header platform={state.platform} status={state.status} />
    <Hero
      packageManager={state.packageManager}
      status={state.status}
      components={state.components}
      {onEnter}
    />
    {#if state.needsAction && state.remedy}
      <Remedy remedy={state.remedy} {onCopyScript} {onRecheck} />
    {/if}
    <Ledger components={state.components} />
    <footer class="flex justify-between items-center gap-4 mt-8 pt-5.5 border-t border-surface-z2">
      <span class="text-2xs text-surface-z5">Bootstrap runs once. The next launch will be quick.</span>
      {#if state.isOk}
        <button data-action="continue" class="btn-solid" onclick={onEnter}>Continue →</button>
      {/if}
    </footer>
  </div>
</div>
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cd app && bun run test:unit -- src/routes/\(health\)/health/HealthView.spec.svelte.ts 2>&1 | tail -20
```

Expected: 6 tests pass including the reactive flip.

- [ ] **Step 5: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add 'app/src/routes/(health)/health/HealthView.svelte' 'app/src/routes/(health)/health/HealthView.spec.svelte.ts'
git commit -m "feat(app): compose HealthView from Header/Hero/Remedy/Ledger"
```

---

## Task 16 — Rewrite `/health/+page.svelte` to use `HealthView`

**Files:**
- Modify: `app/src/routes/(health)/health/+page.svelte`

- [ ] **Step 1: Replace the entire file**

Overwrite `app/src/routes/(health)/health/+page.svelte` with:

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { healthState } from '$lib/health-state.svelte.js';
  import HealthView from './HealthView.svelte';

  function onEnter() { goto('/', { replaceState: true }); }
  function onRecheck() { /* Phase 1a: no-op; Phase 2 wires healthState.recheck() */ }
  function onCopyScript() {
    if (healthState.remedy) navigator.clipboard?.writeText(healthState.remedy.script);
  }
</script>

<HealthView state={healthState} {onEnter} {onRecheck} {onCopyScript} />
```

- [ ] **Step 2: svelte-check**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: 0 new errors. The page no longer imports `bootstrap-state` or `bootstrap.ts`.

- [ ] **Step 3: Full test suite**

```bash
cd app && bun run test:unit 2>&1 | tail -10
```

Expected: every existing test still passes.

- [ ] **Step 4: Commit**

```bash
git add 'app/src/routes/(health)/health/+page.svelte'
git commit -m "feat(app): rewrite /health page to render HealthView from singleton"
```

---

## Task 17 — Delete orphaned bootstrap-state + bootstrap-gates

**Files:**
- Delete: `app/src/lib/bootstrap-state.svelte.ts`
- Delete: `app/src/lib/bootstrap-gates.ts`

Note: `app/src/lib/bootstrap.ts` (the Tauri command wrappers) is **kept** — Phase 2's `HealthTransport` will reuse `getDaemonPort` and the event listeners.

- [ ] **Step 1: Confirm no remaining references**

```bash
cd app && grep -rn "bootstrap-state\|bootstrap-gates" src/ 2>&1
```

Expected: zero results. If anything matches, stop and update the call site to use `healthState` or remove the dead import before deleting.

- [ ] **Step 2: Delete the files**

```bash
cd app && git rm src/lib/bootstrap-state.svelte.ts src/lib/bootstrap-gates.ts
```

- [ ] **Step 3: svelte-check + tests**

```bash
cd app && bun run check 2>&1 | tail -10
cd app && bun run test:unit 2>&1 | tail -10
```

Expected: 0 errors; all tests pass.

- [ ] **Step 4: Commit**

```bash
git commit -m "chore(app): remove orphaned bootstrap-state and bootstrap-gates"
```

---

## Task 18 — Final verification gate

This task does not write code; it verifies Phase 1a hit the bar set in the spec.

- [ ] **Step 1: Full test suite from a clean state**

```bash
cd app && bun run test:unit 2>&1 | tail -10
```

Expected: all tests pass, zero failures.

- [ ] **Step 2: Coverage on the new modules**

```bash
cd app && bun run test:unit -- --coverage 2>&1 | tail -40
```

Expected: 100% line + branch coverage for:
- `src/lib/health-state.svelte.ts`
- `src/routes/(health)/health/Header.svelte`
- `src/routes/(health)/health/Hero.svelte`
- `src/routes/(health)/health/Remedy.svelte`
- `src/routes/(health)/health/Ledger.svelte`
- `src/routes/(health)/health/HealthView.svelte`

If any module is under 100%, add the missing test and loop back — do not move on.

- [ ] **Step 3: svelte-check final pass**

```bash
cd app && bun run check 2>&1 | tail -10
```

Expected: 0 errors, 0 warnings (or unchanged from the pre-flight baseline).

- [ ] **Step 4: Manual smoke**

```bash
cd app && bun run dev
```

Browse to `http://localhost:5173/health` (or whichever port Vite picked). Expected: the page renders the cold-load HealthView — Header reads "macOS · bootstrap" with the "Checking the foundation…" headline, Hero shows the spinner, Ledger shows 5 `pending` rows, no Remedy card, no Continue button. No console errors. Stop the dev server with Ctrl-C.

- [ ] **Step 5: Push the branch**

```bash
git push origin develop
```

Phase 1a is complete when this push succeeds with no CI failures (if a CI is configured) and the manual smoke above behaves as described.

---

## Plan complete

After Task 18, the codebase is in the state described by Section 11 of the spec:

- New: `health-types.ts`, `health-state.svelte.ts` (+ spec), `test-mount.ts`, four sub-components + their specs, `HealthView` composition + spec.
- Rewritten: `(health)/health/+page.svelte`.
- Removed: `bootstrap-state.svelte.ts`, `bootstrap-gates.ts`.
- Untouched and intentional carry-over to Phase 2: `bootstrap.ts`, `appstate.healthReady`, `appstate.setHealthReady()`. These remain orphaned until Phase 2 introduces the transport and moves sessionStorage writes into `HealthState.apply()`.

Open question deferred to Phase 1b (per spec Section 10):

- **Q-1.** The 14 scenarios listed in spec Section 7 are tentative; revisit during Phase 1b kick-off.

Open questions deferred to Phase 2:

- **Q-2.** Source of `state.latest` (GitHub Releases vs daemon endpoint).
- **Q-3.** Upgrade-redirect behavior in `hooks.client.ts`.
