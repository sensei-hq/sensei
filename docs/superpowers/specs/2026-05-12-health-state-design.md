# Health state — design

**Date:** 2026-05-12
**Status:** Draft for review
**Replaces:** `app/src/lib/bootstrap-state.svelte.ts`, `app/src/lib/bootstrap-gates.ts`, the existing `/health` page, and `appState.healthReady` / `appState.setHealthReady()`.

---

## 1. Intent

A single, app-global Svelte 5 state class — `HealthState` — that holds the current state of the sensei ecosystem. It is the bridge between two transports (daemon `/health` REST endpoint, Tauri sidecar `check_and_fix_bootstrap` events) and two UI surfaces today (the `/health` page; the home redirect in `hooks.client.ts` via a sessionStorage cache), plus a Phase-2 upgrade-available redirect.

The existing `BootstrapState` entangles transport, UI ledger details, and lifecycle. That entanglement is what makes "run the app 20 times and not get it right" the only feedback loop. This rewrite separates concerns so each unit is independently testable and the contract between them is explicit.

### Phasing

The work is split into three phases. **This document specifies Phase 1 in full.** Phases 2 and 3 are sketched to confirm the design doesn't paint Phase 1 into a corner.

| Phase | Scope | Verification |
|-------|-------|--------------|
| **1a** | `HealthState` data class + `HealthView` rendering component, types, runtime + compile-time invariants | Unit tests on state class, component tests with hand-built payloads |
| **1b** | `scenarios.ts` registry + parametrized snapshot tests in `HealthView.spec.svelte.ts` that iterate the registry — one snapshot per scenario | All scenario snapshots stable; `vitest -u` is the deliberate review step when output changes |
| **2** | `HealthTransport` interface + real implementation (daemon REST fast path, Tauri sidecar fallback, lazy `latest` fetch), `init()` / `recheck()` lifecycle, sessionStorage cache for synchronous hook reads | Transport tests with mock fetch + mock Tauri; integration test of `init()` → state mutation |
| **3** | `sensei_bootstrap` Rust crate produces the exact `HealthPayload` shape; both daemon `/health` and Tauri command serialize it; old bootstrap event protocol retired | E2E Tauri dev: delete `sensei_dev` DB + sensei files; cold-start app; verify auto-repair → install → setup → all-green |

**Phase 1b scenarios are deferred** until Phase 1a (state + component) is implemented and reviewed. Section 7 captures the scenario shape we've already agreed on; Section 10 lists the open questions to revisit before starting 1b.

---

## 2. Architecture

```
                                  ┌─────────────────────────────────────┐
                                  │  /health/+page.svelte               │
                                  │  reads healthState, renders         │
                                  │  <HealthView state={healthState}/>  │
                                  └─────────────────────────────────────┘
                                                   ▲ reactive
                                                   │
┌────────────────┐                ┌────────────────┴─────────────────┐
│ hooks.client.ts│  reads sync    │  healthState  (singleton)        │      Phase 2 only:
│   reroute      │ ─────────────► │  status · packageManager ·       │      ┌─────────────────────┐
│                │  sessionStg    │  components · remedy · version   │ ◄──► │  HealthTransport    │
└────────────────┘  (Phase 2)     │  · latest                        │      │  fetchHealth()      │
                                  │                                  │      │  resolve(onEvt)     │
                                  │  apply(payload)                  │      │  fetchLatest()      │
                                  │  applyEvent(event)               │      └─────────────────────┘
                                  └──────────────────────────────────┘
                                                   ▲
                                                   │  HealthView + scenario.terminal
                                  ┌────────────────┴─────────────────┐
                                  │  HealthView.spec.svelte.ts       │
                                  │  vitest snapshot per scenario —  │
                                  │  the verifier surface (Phase 1b) │
                                  └──────────────────────────────────┘
```

### Files

```
app/src/lib/
├── health-types.ts                       # types, enums, invariants (Phase 1a)
├── health-state.svelte.ts                # HealthState class + singleton (Phase 1a)
├── health-state.spec.svelte.ts           # state class tests (Phase 1a)
├── health-scenarios.ts                   # scenario registry array (Phase 1b)
└── health-transport.ts                   # HealthTransport interface + impls (Phase 2)

app/src/routes/(health)/health/
├── +page.svelte                          # uses healthState singleton (Phase 1a)
├── HealthView.svelte                     # thin composition: lays out the 4 parts (Phase 1a)
├── HealthView.spec.svelte.ts             # composition test: presence/absence of parts (Phase 1a)
├── Header.svelte                         # 支 tag, h1, sub-prose (Phase 1a)
├── Header.spec.svelte.ts                 # unit + snapshot tests (Phase 1a/1b)
├── Hero.svelte                           # package manager card + Enter button (Phase 1a)
├── Hero.spec.svelte.ts                   # unit + snapshot tests (Phase 1a/1b)
├── Remedy.svelte                         # manual fallback card; receives non-null Remedy (Phase 1a)
├── Remedy.spec.svelte.ts                 # unit + snapshot tests (Phase 1a/1b)
├── Ledger.svelte                         # 5-row component list (Phase 1a)
└── Ledger.spec.svelte.ts                 # unit + snapshot tests (Phase 1a/1b)
```

SvelteKit treats only `+page*` / `+layout*` / `+server*` / `+error*` files as route artifacts inside `routes/`. Anything else (like `Header.svelte`, `HealthView.svelte`) is a regular file the route imports — co-locating components with the route that owns them is idiomatic.

### Deletes (at end of Phase 1)

- `app/src/lib/bootstrap-state.svelte.ts`
- `app/src/lib/bootstrap-gates.ts`
- `appState.healthReady` field and `appState.setHealthReady()` method (the session cache moves into `HealthState`)
- Existing `/health/+page.svelte` is rewritten end-to-end

---

## 3. Types — `health-types.ts`

Pure data, no Svelte/Tauri imports. This is the Rust → TS contract that Phase 3 must match.

```ts
// ── Closed enums ─────────────────────────────────────────────────────────

export type Platform        = 'macos' | 'linux' | 'windows';
export type HealthStatus    = 'checking' | 'resolving' | 'ok' | 'needs-action';
export type ComponentStatus = 'pending' | 'checking' | 'installing' | 'ready' | 'failed';

export type ComponentId      = 'postgres' | 'ollama' | 'sensei' | 'database' | 'daemon';
export type PackageManagerId = 'homebrew' | 'winget';

export const COMPONENT_ORDER: readonly ComponentId[] =
  ['postgres', 'ollama', 'sensei', 'database', 'daemon'] as const;

// ── Component (both ledger rows and package manager use this shape) ──────

export interface Component {
  id: ComponentId | PackageManagerId;
  label: string;             // "PostgreSQL @16", "Homebrew"
  note: string | null;       // "pgvector · sensei tables"
  status: ComponentStatus;
  version: string | null;    // "16.3" when known
  detail: string | null;     // failure detail when status === 'failed'
}

// ── Remedy — opaque strings produced by the Rust crate ───────────────────
// UI renders verbatim. Crate owns wording, platform commands, sudo logic.

export interface Remedy {
  message: string;            // pre-rendered prose for the manual card
  script: string;             // copy-pasteable script, platform-correct
  url: string | null;         // optional informational link
}

// ── Discriminated union: status === 'needs-action' ⇔ remedy !== null ─────

type PayloadBase = {
  version: string;            // current app version
  uptimeSeconds: number;
  platform: Platform;
  packageManager: Component;  // id is PackageManagerId
  components: Component[];    // length 5, ordered per COMPONENT_ORDER
};

export type HealthPayload =
  | (PayloadBase & { status: Exclude<HealthStatus, 'needs-action'>; remedy: null })
  | (PayloadBase & { status: 'needs-action'; remedy: Remedy });

// ── Streaming events (used by scripted scenarios + Phase 2 sidecar) ──────

export type HealthEvent =
  | { kind: 'phase';     phase: Extract<HealthStatus, 'checking' | 'resolving'> }
  | { kind: 'component'; id: ComponentId | PackageManagerId; patch: Partial<Component> }
  | { kind: 'remedy';    remedy: Remedy }
  | { kind: 'report';    payload: HealthPayload };   // terminal
```

### Invariants

| ID | Invariant | Where enforced |
|----|-----------|----------------|
| INV-1 | `status === 'needs-action' ⇔ remedy !== null` | (C) TS discriminated union + (A) `apply()` throws |
| INV-2 | `components.length === 5` and ordered per `COMPONENT_ORDER` | `apply()` throws |
| INV-3 | `packageManager.id ∈ PackageManagerId` and matches platform (`homebrew` for macos/linux, `winget` for windows) | `apply()` throws |
| INV-4 | Unknown `ComponentId` in `applyEvent({ kind: 'component', ... })` throws | `#patch()` throws with the canonical id list in the message |
| INV-5 | Unknown `HealthEvent.kind` is a TS compile error AND a runtime throw | `applyEvent` exhaustive `switch` with `never` check |

INV-2 and INV-3 are runtime guards in `apply()` mirroring what a payload coming off the wire (Phase 2) might violate. INV-1 is the most load-bearing — captured both at compile time and at runtime.

---

## 4. The `HealthState` class — `health-state.svelte.ts`

Pure data holder. **No async, no transport, no sessionStorage, no lifecycle.** Phase 2 adds those.

```ts
import type {
  HealthPayload, HealthEvent, Component, HealthStatus, Platform,
  PackageManagerId, ComponentId, Remedy,
} from './health-types.js';
import { COMPONENT_ORDER } from './health-types.js';

/** Deterministic empty seed. status='checking' so the UI never flashes an "ok" between cold-load and the first apply(). */
export const emptyPayload: HealthPayload = {
  version: '',
  uptimeSeconds: 0,
  platform: 'macos',
  packageManager: { id: 'homebrew', label: 'Homebrew', note: null, status: 'pending', version: null, detail: null },
  components: COMPONENT_ORDER.map((id) => emptyComponent(id)),
  status: 'checking',
  remedy: null,
};

export class HealthState {
  // ── Reactive fields — placeholders; apply(seed ?? emptyPayload) sets real values ──
  status         = $state<HealthStatus>('checking');
  version        = $state<string>('');
  latest         = $state<string | null>(null);
  platform       = $state<Platform>('macos');
  packageManager = $state<Component>(emptyPayload.packageManager);
  components     = $state<Component[]>(emptyPayload.components);
  remedy         = $state<Remedy | null>(null);

  /** Seed defaults to emptyPayload so every instance is validated through apply(). */
  constructor(seed: HealthPayload = emptyPayload) {
    this.apply(seed);
  }

  // ── Derived (getters) ───────────────────────────────────────
  get isOk():        boolean { return this.status === 'ok'; }
  get isBusy():      boolean { return this.status === 'checking' || this.status === 'resolving'; }
  get needsAction(): boolean { return this.status === 'needs-action'; }

  // ── Mutation (only way state changes) ───────────────────────

  /** Replace the entire payload. Enforces INV-1, INV-2, INV-3. */
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

  /** Apply one streaming event. INV-4 and INV-5 throw on unknown shapes. */
  applyEvent(e: HealthEvent): void {
    switch (e.kind) {
      case 'phase':     this.status = e.phase; return;
      case 'component': this.#patch(e.id, e.patch); return;
      case 'remedy':    this.remedy = e.remedy; return;
      case 'report':    this.apply(e.payload); return;
      default: {
        const _exhaustive: never = e;
        throw new Error(`HealthState: unknown event kind ${JSON.stringify(_exhaustive)}`);
      }
    }
  }

  #patch(id: ComponentId | PackageManagerId, patch: Partial<Component>): void {
    if (id === this.packageManager.id) {
      this.packageManager = { ...this.packageManager, ...patch };
      return;
    }
    const idx = this.components.findIndex(c => c.id === id);
    if (idx < 0) {
      throw new Error(
        `HealthState: unknown component id "${id}" — not in [${this.packageManager.id}, ${COMPONENT_ORDER.join(', ')}]`
      );
    }
    const next = this.components.slice();
    next[idx] = { ...next[idx], ...patch };
    this.components = next;
  }
}

/** Singleton — Phase 1 stays in the cold-load state until the page applies a payload. */
export const healthState = new HealthState();

function emptyComponent(id: ComponentId): Component {
  // labels and notes come from the crate in Phase 2; for the empty seed they're trivial placeholders.
  return { id, label: id, note: null, status: 'pending', version: null, detail: null };
}
```

**Properties enforced by construction**
- Constructor pure — no I/O, safe to construct freely in tests.
- `apply()` is the only path to terminal states; same code path used by `report` events and explicit calls.
- Every shape error throws with a message that names the field and the expected value.

---

## 5. UI components — `components/health/`

`HealthView` is now a thin composition. Four sub-components own the actual rendering, each with a narrow prop contract that makes snapshot tests focused and diffs localized.

### 5.1 `HealthView.svelte` — composition only

```svelte
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

<div class="health-view">
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
  <footer>
    <span>Bootstrap runs once. The next launch will be quick.</span>
    {#if state.isOk}<button onclick={onEnter}>Continue →</button>{/if}
  </footer>
</div>
```

The `<footer>` stays inline — it's one button and one line; promoting it to its own component buys nothing.

### 5.2 `Header.svelte`

```ts
interface Props {
  platform: Platform;
  status: HealthStatus;
}
```

Renders the `支 BOOTSTRAP · <platform>` tag, the H1 (`"The foundation holds." / "Setting up your foundation." / "One last step."` keyed off `status`), and the static sub-prose. Pure function of two props. Platform → display label uses `{ macos: 'macOS', linux: 'Linux', windows: 'Windows' }`.

### 5.3 `Hero.svelte`

```ts
interface Props {
  packageManager: Component;     // ready / installing / failed / …
  status: HealthStatus;          // drives sub-copy and the [Enter] button
  components: Component[];       // needed for the "Installing X (i/5)" derived index
  onEnter?: () => void;
}
```

Renders the package-manager card: indicator circle (ring color by status), label + `note`, status sub-copy (`"Detected. All dependencies installed."` / `"Installing <activeLabel> (i/5)"` / `"Couldn't finish automatically."`). `[Enter]` button rendered iff `status === 'ok'`.

`activeIdx` is derived inside `Hero` from `components` — first index whose status is `installing` or `checking`; else the count of `ready` rows. Single deterministic expression, easy to snapshot-test.

### 5.4 `Remedy.svelte`

```ts
interface Props {
  remedy: Remedy;                // non-null — parent renders this only when needsAction
  onCopyScript?: () => void;
  onRecheck?: () => void;
}
```

Renders the manual-fallback card: header bar (`手 Run this in your terminal`), `remedy.message`, `<pre>{remedy.script}</pre>`, `[Copy script]` + `[I've run it · re-check]` buttons. Component never decides whether to be visible — that's the parent's job.

This is why `Remedy` takes a non-nullable `Remedy` prop, not the state: it can't be misused to render with no remedy, and its tests don't need to construct a full `HealthState`.

### 5.5 `Ledger.svelte`

```ts
interface Props {
  components: Component[];       // exactly 5, ordered per COMPONENT_ORDER
}
```

Renders `WHAT THIS RESOLVES` heading + 5 rows. Each row: dot (color by `c.status`), `c.label`, optional `· c.note`, status badge. Failed rows render `c.detail` under the label.

Dot/badge color map (lives in this file as a tiny lookup):

| status | dot/badge color | opacity |
|---|---|---|
| `ready` | success | 1.0 |
| `installing` / `checking` | primary | 1.0 |
| `failed` | primary-dimmed | 1.0 |
| `pending` | surface | 0.55 |

### Binding rules — invariant across all four

- No sub-component imports `HealthState`. They take POJO props derived from it.
- No platform-specific copy or commands anywhere except `Remedy.script` / `Remedy.message`, which are opaque strings from the crate.
- No async, no fetches, no timers, no `$effect`. Pure derivations of props.
- Callbacks are fire-and-forget; sub-components don't track loading or error state for them.

---

## 6. Routes

### `/health/+page.svelte` — the real health page

```svelte
<script lang="ts">
  import { healthState } from '$lib/health-state.svelte.js';
  import HealthView from '$lib/components/HealthView.svelte';
  import { goto } from '$app/navigation';

  function onEnter()   { goto('/', { replaceState: true }); }
  function onRecheck() { /* Phase 1a: no-op stub; Phase 2: healthState.recheck() */ }
  function onCopyScript() {
    if (healthState.remedy) navigator.clipboard?.writeText(healthState.remedy.script);
  }
</script>

<HealthView state={healthState} {onEnter} {onRecheck} {onCopyScript} />
```

In Phase 1a, the singleton stays in its empty/checking state and the page renders the "cold-load" view. Phase 1b's snapshot tests verify every scenario without touching this route. Phase 2 wires `healthState.init()` and the real transports.

### No second route — scenarios are verified via vitest snapshots

Phase 1b does **not** add a `/health/[slug]` route. The verification surface for every scenario is `HealthView.spec.svelte.ts`, which renders `HealthView` with each `scenario.terminal` and takes a vitest snapshot. See Section 8.

This is strictly better than a dev-only route:

- One artifact per scenario lives in version control (`__snapshots__/HealthView.spec.svelte.ts.snap`). Regressions show as diffs in code review, not as something a human has to remember to eyeball.
- No production-route gating to maintain — nothing in the route tree depends on `import.meta.env.DEV`.
- The same `SCENARIOS` array drives both the snapshot tests and (later) the `HealthState` invariant tests over the registry.
- If a future need calls for visual eyeballing, that can be added as a separate harness without touching `app/src/routes/`.

---

## 7. Scenario registry — `health-scenarios.ts` (Phase 1b, deferred)

The full design is **deferred** until Phase 1a is implemented and reviewed. The shape we've agreed on, for reference:

```ts
export interface Scenario {
  slug: string;                 // URL-safe id
  label: string;                // human-readable
  terminal: HealthPayload;      // final state
}

export const SCENARIOS: readonly Scenario[];                       // ordered, immutable
export const SCENARIO_BY_SLUG: Record<string, Scenario>;
```

### Scenarios committed to (will be expanded in 1b design)

Captured here so 1a doesn't paint us into a corner. Each row will become a `Scenario` entry in 1b.

| Slug | Status | Platform | Notes |
|---|---|---|---|
| `all-green-mac` | ok | macos | every component ready |
| `all-green-linux` | ok | linux | every component ready |
| `all-green-windows` | ok | windows | every component ready, winget |
| `checking` | checking | macos | cold-load seed |
| `no-brew-mac` | needs-action | macos | homebrew.failed, remedy from crate |
| `no-brew-linux` | needs-action | linux | homebrew.failed |
| `no-winget-windows` | needs-action | windows | winget.failed |
| `bundle-error-mac` | needs-action | macos | homebrew.ready, components failed mid-bundle |
| `permission-mac` | needs-action | macos | remedy mentions sudo |
| `postgres-failed-mac` | needs-action | macos | postgres.failed only |
| `ollama-failed-mac` | needs-action | macos | ollama.failed only |
| `sensei-binaries-failed-mac` | needs-action | macos | sensei.failed only |
| `database-failed-mac` | needs-action | macos | database.failed only |
| `daemon-failed-mac` | needs-action | macos | daemon.failed only |

Phase 1b will:
1. Build component factories (`pg('ready')`, `ollama('failed', detail)` …).
2. Write the 14 scenarios above plus any that emerge from review.
3. Add a parametrized `it.each(SCENARIOS)` snapshot test in `HealthView.spec.svelte.ts` — one inline `toMatchSnapshot()` per scenario. Initial run records all snapshots; subsequent runs diff against them. `vitest -u` is the explicit "I have reviewed and accepted this change" gesture.
4. Validate runtime invariants over the registry (every `needs-action` has a `Remedy`, every `ok` has all components `ready`, no duplicate slugs).

---

## 8. Test plan — Phase 1a (red-green TDD, 100% coverage)

### `health-state.spec.svelte.ts`

Each `it` block is one red→green cycle.

**Construction**
- `new HealthState()` starts with `status='checking'`, components in `COMPONENT_ORDER` order, all pending.
- `new HealthState(payload)` applies the seed and reflects every field.

**`apply()` — happy paths**
- applying an `ok` payload sets `status='ok'`, `remedy` null, all components updated.
- applying a `needs-action` payload sets `status='needs-action'` and `remedy` non-null.
- applying a `resolving` payload sets `status='resolving'` and clears any previous `remedy`.
- applying after applying again replaces fields (no merging).

**`apply()` — invariant violations throw (INV-1, INV-2, INV-3)**
- `needs-action` with `remedy: null` throws.
- non-`needs-action` with `remedy: <Remedy>` throws (TS prevents this; runtime test uses `as any`).
- `components.length !== 5` throws.
- `components[i].id` mismatch with `COMPONENT_ORDER[i]` throws.
- `platform='macos'` with `packageManager.id='winget'` throws (and the linux/windows symmetric cases).

**`applyEvent({ kind: 'phase' })`**
- `'checking'` → `status='checking'`.
- `'resolving'` → `status='resolving'`.

**`applyEvent({ kind: 'component', ... })`**
- patching a known ledger id updates only that component (others unchanged by deep-equal).
- patching `packageManager.id` updates only the package manager.
- patching an unknown id throws with the canonical id list in the message (INV-4).
- partial patch leaves un-patched fields intact.

**`applyEvent({ kind: 'remedy' })`**
- replaces `remedy` regardless of previous status.

**`applyEvent({ kind: 'report' })`**
- equivalent to calling `apply()` with the same payload (one assertion that proves the path).

**`applyEvent({ kind: 'bogus' as any })`**
- throws (INV-5). TS compile error covers the safe path.

**Derived getters**
- `isOk`, `isBusy`, `needsAction` flip correctly across each `HealthStatus` value (4 cases × 3 getters).

### Per-sub-component tests

Each sub-component owns its `.spec.svelte.ts` file. Tests are organized in two layers per file:

**Layer 1 — targeted assertions (Phase 1a).** Hand-built POJO props that exercise the prop contract directly. Layer 1 is what turns red→green during Phase 1a.

**Layer 2 — snapshot per scenario (Phase 1b).** `it.each(SCENARIOS)` projects the relevant fields from each `scenario.terminal` and runs `toMatchSnapshot()`. One snapshot file per sub-component, scoped to that component's render only — so a copy tweak in `Header` doesn't churn `Hero` or `Ledger` snapshots.

#### `Header.spec.svelte.ts`

- Renders the platform-mapped label for each of `macos`, `linux`, `windows`.
- Renders the correct H1 for each of `ok`, `checking`, `resolving`, `needs-action`.
- Layer 2: `it.each(SCENARIOS)('Header — $slug', s => snapshot)` — projects `{ platform, status }`.

#### `Hero.spec.svelte.ts`

- Renders `packageManager.label` and `packageManager.note`.
- Indicator circle class/color reflects `packageManager.status` × `status`.
- `[Enter]` button present iff `status === 'ok'`; calls `onEnter` on click.
- "Installing X (i/5)" indicator uses the first `installing`/`checking` row from `components`; falls back to ready-count progress otherwise.
- Layer 2: `it.each(SCENARIOS)('Hero — $slug', s => snapshot)` — projects `{ packageManager, status, components }`.

#### `Remedy.spec.svelte.ts`

- Renders `remedy.message` and `remedy.script` verbatim inside `<pre>`.
- `[Copy script]` button calls `onCopyScript`.
- `[re-check]` button calls `onRecheck`.
- Renders `remedy.url` link iff non-null.
- Layer 2: `it.each(SCENARIOS.filter(s => s.terminal.remedy))('Remedy — $slug', s => snapshot)` — projects `remedy` only, skipping scenarios where it's null.

#### `Ledger.spec.svelte.ts`

- Renders 5 rows in `COMPONENT_ORDER` order (assert on the rendered text in order).
- Per-status × per-row matrix: for each of `pending`, `checking`, `installing`, `ready`, `failed`, set one row to that status and assert dot color / badge text / opacity.
- `failed` row renders `c.detail` text.
- Throws / renders an error fallback if `components.length !== 5`. (Defensive — should never happen because state-level INV-2 rules it out, but the component is its own boundary.)
- Layer 2: `it.each(SCENARIOS)('Ledger — $slug', s => snapshot)` — projects `components`.

#### `HealthView.spec.svelte.ts` — composition only

Doesn't snapshot full output — that would just duplicate the sub-component snapshots. Asserts the wiring:

- All four sub-components mount.
- `Remedy` is present iff `state.needsAction && state.remedy !== null`.
- `[Continue →]` footer button is present iff `state.isOk`; calls `onEnter`.
- Reactive: mutating `state.status` through `applyEvent` flips `Remedy` and `Continue →` visibility without remounting other parts.

### Snapshot stability — applies to all snapshot files

- No `Date.now()`, random ids, or absolute paths in any sub-component (audited during Phase 1a).
- Test renders use fixed POJO props — `uptimeSeconds: 0`, fixed `version`, no `Math.random()`.
- Snapshot diffs in PRs are the explicit review surface; `vitest -u` is the deliberate acceptance gesture.

### Coverage target

`bun run test:unit -- --coverage` must show 100% line + branch coverage for `health-state.svelte.ts` and `HealthView.svelte`. This is enforceable because Phase 1a has no I/O and no async — every line is reachable from a unit test.

---

## 9. Out of scope for Phase 1, captured here

These are noted to confirm the design supports them without committing to specifics:

- **Phase 2 — `HealthTransport`.** Interface: `fetchHealth()`, `resolve(onEvent)`, `fetchLatest()`. Real impl uses `fetch('/health')` and `tauriInvoke('check_and_fix_bootstrap')` + event listener. Tests for the transport itself use `vi.stubGlobal('fetch', ...)` and `vi.mock('@tauri-apps/api/...')`.
- **Phase 2 — lifecycle methods on `HealthState`.** `init()` (idempotent, fires once on app load), `recheck()` (forces a re-check, clears session cache), `dispose()` (tears down event listeners). All three live on the class and only interact with the transport — no extra concepts introduced.
- **Phase 2 — sessionStorage cache.** Key `sensei:health`, value `'ready'` iff `status === 'ok'`. `hooks.client.ts` reroute reads this synchronously to avoid redirecting to `/health` on every navigation. The cache write happens inside `apply()`. Removes the `appState.setHealthReady()` / `appState.healthReady` pair.
- **Phase 2 — `latest` lazy fetch.** Source (GitHub Releases vs daemon endpoint) is the only open Phase-2 question.
- **Phase 3 — Rust crate alignment.** `sensei_bootstrap` adds `HealthPayload` (serde) as its public output type. Daemon `/health` handler returns it directly; Tauri `check_and_fix_bootstrap` emits `HealthEvent`s and a final `HealthPayload` via the `report` event. Existing per-gate event protocol is retired.
- **Phase 3 — E2E verification.** Delete `sensei_dev` DB and sensei files; cold-start app; visually verify the bootstrap walks postgres → ollama → sensei → database → daemon → all-green.

---

## 10. Open questions

| ID | Question | Resolution path |
|----|----------|-----------------|
| Q-1 | Are 14 scenarios the right Phase-1b scope, or do we need more? | Re-evaluate during Phase 1b kick-off (after 1a is merged). |
| Q-2 | Where does `latest` come from in Phase 2 — GitHub Releases API, or a daemon `/api/version/latest` endpoint? | Decide at Phase 2 kick-off. Phase 1a sets the field to `null` and never touches it. |
| Q-3 | Upgrade-redirect behavior: when `latest > version`, does `hooks.client.ts` redirect to `/upgrade` unconditionally, or only on initial cold load? | Decide at Phase 2 kick-off. Field exists in Phase 1a; behavior is not implemented. |

---

## 11. Build order for Phase 1a (red-green)

1. Write `health-types.ts` with the discriminated union and `COMPONENT_ORDER`.
2. Write the full test file `health-state.spec.svelte.ts` (every `it` red).
3. Write `health-state.svelte.ts` minimum to turn each test green, one at a time.
4. Write component tests for `HealthView.svelte` (every `it` red).
5. Write `HealthView.svelte` minimum to turn each component test green.
6. Wire `/health/+page.svelte` to render `<HealthView state={healthState} />`.
7. Delete `bootstrap-state.svelte.ts`, `bootstrap-gates.ts`, related dead code in `appstate.svelte.ts`.
8. Run `bun run check`, `bun run test:unit -- --coverage`, manually browse `/health` (will show the cold-load state).

At step 8: 100% coverage on `health-state.svelte.ts` and `HealthView.svelte`, zero TS errors, zero lint errors. Phase 1a complete.
