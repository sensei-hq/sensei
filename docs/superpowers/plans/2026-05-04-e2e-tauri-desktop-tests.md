# Tauri Desktop E2E Tests Implementation Plan

> **Note (2026-05-12):** References to `SENSEI_MODE=dev` env var are outdated.
> Mode is now compile-time via `--features dev` Cargo flag. See `docs/design/daemon/debug-vs-release.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all browser-mode (mocked IPC) E2E tests with full Tauri desktop tests that build and launch the real `Sensei.app` binary automatically against the `sensei-dev` database.

**Architecture:** A Playwright `globalSetup` script builds both the `senseid` daemon binary and `Sensei.app` (debug + `e2e-testing` Cargo feature), swaps the `~/.local/bin/senseid` symlink to the debug binary, launches `Sensei.app` with dev environment variables, and waits for the `tauri-plugin-playwright` Unix socket at `/tmp/tauri-playwright.sock`. Tests connect via that socket. `globalTeardown` kills the app, stops senseid, and restores the symlink to the release binary.

**Tech Stack:** Playwright, `@srsholmes/tauri-playwright`, TypeScript (ESM), Cargo / `cargo tauri build`, Tauri 2, SvelteKit 2

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `app/e2e/globalSetup.ts` | **Create** | Build both binaries, swap symlink, launch app, wait for socket |
| `app/e2e/globalTeardown.ts` | **Create** | Kill app, stop senseid, restore symlink |
| `app/e2e/playwright.config.ts` | **Replace** | Single `tauri` project, globalSetup/globalTeardown, no `webServer` |
| `app/e2e/fixtures.ts` | **Replace** | Socket-only fixture — no `ipcMocks`, no `devUrl` |
| `app/e2e/helpers.ts` | **Modify** | Update `navigateTo` with fallback for built app (no Vite) |
| `app/e2e/tests/boot-flow.spec.ts` | **Replace** | Real bootstrap: gates visible, page advances |
| `app/e2e/tests/db-setup.spec.ts` | **Replace** | Real DB gate state: terminal state + remedy UI |
| `app/e2e/tests/setup-wizard.spec.ts` | **No change** | Already imports from `../fixtures` with no mock deps |
| `app/e2e/fixtures-db-missing.ts` | **Delete** | Stateful mock — not needed with real app |
| `app/e2e/setup/wizard.spec.ts` | **Delete** | Old daemon-driven suite — superseded |
| `app/package.json` | **Modify** | Add `test:e2e:desktop` script |

---

## Task 1: Create `e2e/globalSetup.ts`

**Files:**
- Create: `app/e2e/globalSetup.ts`

### Context

The setup module runs once before all tests. It builds the `senseid` daemon and `Sensei.app`, swaps the symlink, launches the app, and waits for the Unix socket. Playwright discovers it via `globalSetup: './globalSetup.ts'` in the config.

Key paths (all relative to `app/e2e/`):
- `DAEMON_REPO` = `app/e2e/../../daemon` = `daemon/`
- `APP_REPO` = `app/e2e/..` = `app/`
- `SENSEID_DEBUG` = `daemon/target/debug/senseid`
- `SENSEID_RELEASE` = `daemon/target/release/senseid`
- `APP_BINARY` = `app/src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop`
- `SYMLINK` = `~/.local/bin/senseid`

Uses `execFileSync` (no shell — no injection risk) and `fs.symlinkSync` for the symlink swap. The Tauri build runs from `app/src-tauri/` because `tauri.conf.json` lives there (required by `cargo tauri build`).

- [ ] **Step 1: Create `app/e2e/globalSetup.ts`**

```typescript
import { execFileSync, spawn } from 'child_process';
import { existsSync, symlinkSync, unlinkSync, writeFileSync } from 'fs';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

const DAEMON_REPO     = resolve(__dirname, '../../daemon');
const APP_REPO        = resolve(__dirname, '..');
const SENSEID_DEBUG   = join(DAEMON_REPO, 'target/debug/senseid');
const APP_BINARY      = join(
  APP_REPO,
  'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop',
);
const SOCKET   = '/tmp/tauri-playwright.sock';
const PID_FILE = '/tmp/sensei-e2e-pid';
const HOME     = process.env.HOME ?? '';
const SYMLINK  = join(HOME, '.local/bin/senseid');

function sleep(ms: number): Promise<void> {
  return new Promise(r => setTimeout(r, ms));
}

async function waitForSocket(socketPath: string, timeoutMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (existsSync(socketPath)) return;
    await sleep(500);
  }
  throw new Error(`Timed out waiting for ${socketPath} (${timeoutMs}ms)`);
}

function swapSymlink(target: string, link: string): void {
  try { unlinkSync(link); } catch { /* did not exist */ }
  symlinkSync(target, link);
}

export default async function globalSetup(): Promise<void> {
  // 1. Build senseid daemon (debug)
  console.log('[globalSetup] Building senseid...');
  execFileSync('cargo', ['build', '-p', 'senseid'], {
    cwd: DAEMON_REPO,
    stdio: 'inherit',
  });

  // 2. Build Sensei.app (debug + e2e-testing feature)
  console.log('[globalSetup] Building Sensei.app...');
  execFileSync('cargo', ['tauri', 'build', '--debug', '--features', 'e2e-testing'], {
    cwd: join(APP_REPO, 'src-tauri'),
    stdio: 'inherit',
  });

  // 3. Stop any running senseid before swapping symlink
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }
  await sleep(500);

  // 4. Swap symlink to debug binary
  swapSymlink(SENSEID_DEBUG, SYMLINK);

  // 5. Launch Sensei.app with dev env vars
  console.log('[globalSetup] Launching Sensei.app...');
  const proc = spawn(APP_BINARY, [], {
    env: { ...process.env, SENSEI_MODE: 'dev', SENSEI_DB_NAME: 'sensei-dev' },
    detached: true,
    stdio: 'ignore',
  });
  proc.unref();
  writeFileSync(PID_FILE, String(proc.pid));

  // 6. Wait for tauri-plugin-playwright socket (up to 60 s)
  console.log('[globalSetup] Waiting for Tauri socket...');
  await waitForSocket(SOCKET, 60_000);
  console.log('[globalSetup] Socket ready — tests may begin.');
}
```

- [ ] **Step 2: Type-check**

Run from `app/`:
```
npx svelte-kit sync && npx tsc --noEmit --project tsconfig.json
```
Expected: no errors in `e2e/globalSetup.ts`.

- [ ] **Step 3: Commit**

```
cd /Users/Jerry/Developer/sensei/app
git add e2e/globalSetup.ts
git commit -m "feat(e2e): add globalSetup — build, launch Sensei.app, wait for socket"
```

---

## Task 2: Create `e2e/globalTeardown.ts`

**Files:**
- Create: `app/e2e/globalTeardown.ts`

### Context

Teardown runs once after all tests complete. It kills the Sensei.app process by PID, stops senseid, and restores the symlink to the release binary so the production installation is not left pointing at a debug build.

- [ ] **Step 1: Create `app/e2e/globalTeardown.ts`**

```typescript
import { execFileSync } from 'child_process';
import { existsSync, readFileSync, symlinkSync, unlinkSync } from 'fs';
import { resolve, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

const DAEMON_REPO     = resolve(__dirname, '../../daemon');
const SENSEID_RELEASE = join(DAEMON_REPO, 'target/release/senseid');
const PID_FILE        = '/tmp/sensei-e2e-pid';
const HOME            = process.env.HOME ?? '';
const SYMLINK         = join(HOME, '.local/bin/senseid');

function swapSymlink(target: string, link: string): void {
  try { unlinkSync(link); } catch { /* did not exist */ }
  symlinkSync(target, link);
}

export default async function globalTeardown(): Promise<void> {
  // 1. Kill Sensei.app by saved PID
  if (existsSync(PID_FILE)) {
    const pid = readFileSync(PID_FILE, 'utf8').trim();
    try { process.kill(parseInt(pid, 10)); } catch { /* already exited */ }
    unlinkSync(PID_FILE);
  }

  // 2. Stop senseid daemon
  try { execFileSync('/usr/bin/pkill', ['-x', 'senseid'], { stdio: 'ignore' }); } catch { /* not running */ }

  // 3. Restore symlink to release binary
  if (existsSync(SENSEID_RELEASE)) {
    swapSymlink(SENSEID_RELEASE, SYMLINK);
  }
}
```

- [ ] **Step 2: Type-check**

```
npx tsc --noEmit --project tsconfig.json
```
Expected: no errors in `e2e/globalTeardown.ts`.

- [ ] **Step 3: Commit**

```
git add e2e/globalTeardown.ts
git commit -m "feat(e2e): add globalTeardown — kill app, restore senseid symlink"
```

---

## Task 3: Replace `e2e/playwright.config.ts`

**Files:**
- Modify: `app/e2e/playwright.config.ts`

### Context

Remove the `browser` project and `webServer` block entirely. Add `globalSetup` and `globalTeardown` references. Increase `timeout` to 60 s to accommodate real app startup. Keep `workers: 1` — WKWebView has a single window.

- [ ] **Step 1: Replace `app/e2e/playwright.config.ts` with this content**

```typescript
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  retries: 0,
  // WKWebView shares one window — parallel workers would race on the same UI.
  workers: 1,
  globalSetup:    './globalSetup.ts',
  globalTeardown: './globalTeardown.ts',
  projects: [
    { name: 'tauri', use: { mode: 'tauri' } },
  ],
  // No webServer — Vite is not used; app is pre-built by globalSetup.
});
```

- [ ] **Step 2: Verify type-check passes**

```
npx tsc --noEmit --project tsconfig.json
```
Expected: no type errors.

- [ ] **Step 3: Commit**

```
git add e2e/playwright.config.ts
git commit -m "feat(e2e): replace playwright.config — tauri-only project, globalSetup/Teardown"
```

---

## Task 4: Replace `e2e/fixtures.ts`

**Files:**
- Modify: `app/e2e/fixtures.ts`

### Context

Strip out `ipcMocks` (the mock IPC layer) and `devUrl` (Vite dev server). Keep only `mcpSocket` — the socket is opened by `tauri-plugin-playwright` in the running app.

- [ ] **Step 1: Replace `app/e2e/fixtures.ts` with this content**

```typescript
import { createTauriTest } from '@srsholmes/tauri-playwright';

export const { test, expect } = createTauriTest({
  mcpSocket: '/tmp/tauri-playwright.sock',
});
```

- [ ] **Step 2: Verify type-check**

```
npx tsc --noEmit --project tsconfig.json
```
Expected: no errors.

- [ ] **Step 3: Commit**

```
git add e2e/fixtures.ts
git commit -m "feat(e2e): replace fixtures — socket-only, remove ipcMocks and devUrl"
```

---

## Task 5: Update `e2e/helpers.ts` — navigateTo fallback for built app

**Files:**
- Modify: `app/e2e/helpers.ts` (the `navigateTo` function, lines 122–137)

### Context

The current `navigateTo` imports SvelteKit's `goto()` via `/node_modules/@sveltejs/kit/src/runtime/app/navigation.js`. This path is served by Vite in dev mode only — it does **not** exist in the built Tauri app bundle. Add a try/catch fallback that uses an anchor click to trigger SvelteKit's client-side router. SvelteKit intercepts same-origin `<a>` clicks and performs pushState navigation with no full page reload — satisfying the WKWebView constraint.

- [ ] **Step 1: Replace the `navigateTo` function in `app/e2e/helpers.ts`**

Find and replace the `navigateTo` function (lines 122–137). The new version:

```typescript
/**
 * Navigate to a route using SvelteKit's client-side router.
 *
 * WKWebView (Tauri/WebKit) breaks after a full page reload — the initial
 * SvelteKit mount produces empty DOM. Using SvelteKit's own goto() avoids
 * the reload entirely, so components render correctly.
 *
 * Dev mode (Vite running): imports goto() via node_modules URL.
 * Built Tauri app (no Vite): falls back to anchor click, which SvelteKit's
 * router intercepts for client-side pushState navigation (no reload).
 */
export async function navigateTo(
  tauriPage: { evaluate: (script: string) => Promise<unknown> },
  route: string,
): Promise<void> {
  await tauriPage.evaluate(`
    (async function() {
      await new Promise(r => setTimeout(r, 200));
      try {
        var nav = await import('/node_modules/@sveltejs/kit/src/runtime/app/navigation.js');
        await nav.goto(${JSON.stringify(route)});
      } catch {
        // Built Tauri app: anchor click triggers SvelteKit client-side routing
        var a = document.createElement('a');
        a.href = ${JSON.stringify(route)};
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
      }
    })()
  `);
  await sleep(800);
}
```

- [ ] **Step 2: Type-check**

```
npx tsc --noEmit --project tsconfig.json
```
Expected: no errors.

- [ ] **Step 3: Commit**

```
git add e2e/helpers.ts
git commit -m "fix(e2e): navigateTo falls back to anchor click in built Tauri app"
```

---

## Task 6: Replace `e2e/tests/boot-flow.spec.ts`

**Files:**
- Modify: `app/e2e/tests/boot-flow.spec.ts`

### Context

Replace the existing tests (which tested direct routes in mocked IPC) with three real-app assertions against the `/health` bootstrap page:
1. `.bootstrap-page` is visible
2. At least one `.gate-row` is rendered
3. Auto-advance to `/setup/welcome` or `/observatory` within 30 s — if it doesn't advance, staying on `/health` is also a pass (environment-dependent)

- [ ] **Step 1: Replace `app/e2e/tests/boot-flow.spec.ts` with this content**

```typescript
/**
 * Boot flow E2E tests — real Sensei.app, real IPC.
 *
 * Tests the /health bootstrap page against the running app.
 * App is launched by globalSetup with SENSEI_MODE=dev / SENSEI_DB_NAME=sensei-dev.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

test.describe('Boot flow', () => {
  test('health page loads', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    await expect(tauriPage.locator('.bootstrap-page')).toBeVisible({ timeout: 10_000 });
  });

  test('bootstrap gates are visible', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    const gates = tauriPage.locator('.gate-row');
    await expect(gates.first()).toBeVisible({ timeout: 10_000 });
  });

  test('page advances to setup when bootstrap completes', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    // If all gates become ready within 30 s the page auto-advances.
    // If gates are still pending (slow environment), staying on /health is also a pass.
    try {
      await tauriPage.waitForURL(/\/(setup\/welcome|observatory)/, { timeout: 30_000 });
    } catch {
      const url = await tauriPage.url();
      expect(url).toMatch(/\/health/);
    }
  });
});
```

- [ ] **Step 2: Type-check**

```
npx tsc --noEmit --project tsconfig.json
```
Expected: no errors.

- [ ] **Step 3: Commit**

```
git add e2e/tests/boot-flow.spec.ts
git commit -m "feat(e2e): rewrite boot-flow spec — real bootstrap page assertions"
```

---

## Task 7: Replace `e2e/tests/db-setup.spec.ts`

**Files:**
- Modify: `app/e2e/tests/db-setup.spec.ts`

### Context

The current file uses `dbMissingTest` from `fixtures-db-missing.ts` (stateful IPC mock). Replace with two real-app tests:
1. The database gate's `.status-pill` reaches a terminal state (`ready` or `blocked`) within 20 s
2. If the gate is `blocked`, the `.remedy` element is visible

Uses `.filter({ hasText: /database/i })` to isolate the database gate row from the other gate rows.

- [ ] **Step 1: Replace `app/e2e/tests/db-setup.spec.ts` with this content**

```typescript
/**
 * Database gate E2E tests — real Sensei.app, real IPC.
 *
 * Tests the database gate on the /health bootstrap page.
 * Requires PostgreSQL running locally to reach 'ready'.
 * If PostgreSQL is absent the gate reaches 'blocked' — both are valid terminal states.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

test.describe('Bootstrap — database gate', () => {
  test('database gate reaches a terminal state', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');

    const dbPill = tauriPage
      .locator('.gate-row')
      .filter({ hasText: /database/i })
      .locator('.status-pill');

    // Wait up to 20 s for the pill to leave transient states
    await expect(dbPill).not.toContainText(/waiting|checking/, { timeout: 20_000 });
  });

  test('page does not crash when DB gate is blocked', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');

    const dbPill = tauriPage
      .locator('.gate-row')
      .filter({ hasText: /database/i })
      .locator('.status-pill');

    // Determine final state (allow up to 20 s)
    await expect(dbPill).not.toContainText(/waiting|checking/, { timeout: 20_000 });

    const pillText = await dbPill.textContent();
    if (pillText?.includes('blocked')) {
      // Remedy UI must be visible when gate is blocked
      await expect(tauriPage.locator('.remedy')).toBeVisible();
    }
    // If 'ready', no remedy shown — that is fine
  });
});
```

- [ ] **Step 2: Type-check**

```
npx tsc --noEmit --project tsconfig.json
```
Expected: no errors.

- [ ] **Step 3: Commit**

```
git add e2e/tests/db-setup.spec.ts
git commit -m "feat(e2e): rewrite db-setup spec — real database gate state assertions"
```

---

## Task 8: Delete dead files and update `package.json`

**Files:**
- Delete: `app/e2e/fixtures-db-missing.ts`
- Delete: `app/e2e/setup/wizard.spec.ts`
- Modify: `app/package.json`

### Context

`fixtures-db-missing.ts` is the stateful mock fixture — no longer needed. `setup/wizard.spec.ts` is an old daemon-driven suite superseded by `tests/setup-wizard.spec.ts`. Add `test:e2e:desktop` script that runs the `tauri` project against the new config.

- [ ] **Step 1: Delete dead files**

```
cd /Users/Jerry/Developer/sensei/app
git rm e2e/fixtures-db-missing.ts
git rm e2e/setup/wizard.spec.ts
```

If `e2e/setup/` is now empty:
```
rmdir e2e/setup
```

- [ ] **Step 2: Add `test:e2e:desktop` to `app/package.json`**

In `app/package.json`, add this entry to the `"scripts"` object (after `"test:e2e"`):

```
"test:e2e:desktop": "playwright test --config e2e/playwright.config.ts --project=tauri",
```

The full scripts block after the change:

```json
"scripts": {
  "dev": "vite dev",
  "build": "vite build",
  "preview": "vite preview",
  "tauri": "tauri",
  "tauri:dev": "SENSEI_DB_NAME=sensei-dev cargo build --manifest-path src-tauri/Cargo.toml && SENSEI_DB_NAME=sensei-dev tauri dev",
  "tauri:build": "tauri build",
  "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
  "check:watch": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json --watch",
  "test:unit": "vitest run",
  "test:e2e": "npx playwright test --config e2e/playwright.config.ts --project=tauri",
  "test:e2e:desktop": "playwright test --config e2e/playwright.config.ts --project=tauri",
  "test:sidecar": "cd src-tauri && cargo test --test bootstrap_integration",
  "test:playwright": "playwright test --config e2e/playwright.config.ts"
}
```

- [ ] **Step 3: Verify no broken imports**

```
npx tsc --noEmit --project tsconfig.json
```
Expected: no "cannot find module 'fixtures-db-missing'" errors (all imports of that module are now gone).

- [ ] **Step 4: Commit**

```
git add package.json
git commit -m "feat(e2e): delete mock fixtures, add test:e2e:desktop script"
```

---

## Task 9: Smoke run — verify full suite

**Files:** none changed

### Context

Run the full suite to confirm globalSetup builds and launches the app, all three spec files execute against the real app, and globalTeardown cleans up correctly. First run is slow (full Cargo build); subsequent runs are incremental.

**Prerequisites before running:**
- PostgreSQL running locally (for database gate to resolve to `ready`)
- `~/.local/bin/senseid` symlink exists (points at some `senseid` binary currently)
- `cargo` is in `PATH` and `cargo-tauri` is installed (`cargo install tauri-cli --version 2`)
- `daemon/target/release/senseid` exists (teardown needs it to restore symlink)

- [ ] **Step 1: Run the desktop E2E suite**

```
cd /Users/Jerry/Developer/sensei/app
npm run test:e2e:desktop
```

Expected output:
```
[globalSetup] Building senseid...
[globalSetup] Building Sensei.app...
[globalSetup] Launching Sensei.app...
[globalSetup] Waiting for Tauri socket...
[globalSetup] Socket ready — tests may begin.

Running 20 tests using 1 worker

  ✓  tauri > Boot flow > health page loads
  ✓  tauri > Boot flow > bootstrap gates are visible
  ✓  tauri > Boot flow > page advances to setup when bootstrap completes
  ✓  tauri > Bootstrap — database gate > database gate reaches a terminal state
  ✓  tauri > Bootstrap — database gate > page does not crash when DB gate is blocked
  ✓  tauri > Setup Wizard — Welcome > renders welcome page with hero text
  ... (remaining setup-wizard tests)

  20 passed
```

- [ ] **Step 2: If a test fails — triage guide**

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| globalSetup times out waiting for socket | `e2e-testing` feature not compiled in, or app crashed on launch | Verify the build includes the feature; launch the binary manually and check for crash |
| `cargo tauri build` fails with "command not found" | `cargo-tauri` not installed | Run `cargo install tauri-cli --version 2` first |
| `cargo tauri build` fails with `beforeBuildCommand` error (bun not found) | Tauri resolves `beforeBuildCommand` cwd differently | Change `execFileSync` cwd from `join(APP_REPO, 'src-tauri')` to `APP_REPO` and change command to `['npx', 'tauri', 'build', '--debug', '--features', 'e2e-testing']` |
| `.bootstrap-page` not found | CSS selector changed | Inspect the `/health` route component for the actual wrapper class |
| `navigateTo` causes blank WebView | SvelteKit router not intercepting anchor click | Add `bubbles: true` to the click event: `a.dispatchEvent(new MouseEvent('click', { bubbles: true }))` |
| `.gate-row` not found in db-setup tests | Health page HTML structure changed | Check `.gate-row` and `.status-pill` selectors in the health route Svelte component |

- [ ] **Step 3: Commit any fixes**

```
git add -A
git commit -m "fix(e2e): smoke run adjustments"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Covered by |
|-----------------|------------|
| Build `senseid` (cargo build -p senseid) | Task 1 |
| Build `Sensei.app` (debug + e2e-testing) | Task 1 |
| Stop running senseid before symlink swap | Task 1 |
| Swap symlink to debug binary | Task 1 |
| Launch with `SENSEI_MODE=dev`, `SENSEI_DB_NAME=sensei-dev` | Task 1 |
| Wait for `/tmp/tauri-playwright.sock` (60 s timeout) | Task 1 |
| Kill app by PID on teardown | Task 2 |
| Restore symlink to release on teardown | Task 2 |
| Socket-only fixtures (no `ipcMocks`, no `devUrl`) | Task 4 |
| Single `tauri` project, no `webServer` | Task 3 |
| `globalSetup` / `globalTeardown` in config | Task 3 |
| `timeout: 60_000`, `workers: 1` | Task 3 |
| `boot-flow.spec.ts` — `.bootstrap-page` visible | Task 6 |
| `boot-flow.spec.ts` — `.gate-row` visible | Task 6 |
| `boot-flow.spec.ts` — advance or stay, both pass | Task 6 |
| `db-setup.spec.ts` — terminal state check | Task 7 |
| `db-setup.spec.ts` — remedy visible when blocked | Task 7 |
| Delete `fixtures-db-missing.ts` | Task 8 |
| Delete `setup/wizard.spec.ts` | Task 8 |
| Add `test:e2e:desktop` npm script | Task 8 |
| `navigateTo` works with built app (no Vite) | Task 5 |

All requirements covered. No TBDs or placeholders. Type names and function signatures are consistent across all tasks.
