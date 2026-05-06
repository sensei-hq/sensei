# Tauri Desktop E2E Tests — Design Spec

## Goal

Replace all browser-mode (mocked IPC) E2E tests with full Tauri desktop tests that run against the real compiled app using the `sensei-dev` database.

## Architecture

### App under test

Build `Sensei.app` in debug mode with the `e2e-testing` cargo feature, which activates `tauri-plugin-playwright`. The plugin opens a Unix socket at `/tmp/tauri-playwright.sock` that Playwright uses to drive the WebView.

Launch the binary with:
- `SENSEI_MODE=dev` — app operates in dev mode
- `SENSEI_DB_NAME=sensei-dev` — bootstrap targets the `sensei-dev` database

### senseid binary

`senseid` is the background daemon started by the app during bootstrap. The app finds it via `~/.local/bin/senseid` (symlink). Tests point the symlink at the freshly-built debug binary so the app picks up the correct version.

### Lifecycle

| Phase | Action |
|-------|--------|
| globalSetup | 1. Build `senseid` (debug): `cargo build -p senseid` in daemon repo |
| | 2. Build `Sensei.app` (debug + e2e): `cargo tauri build --debug --features e2e-testing` in app repo |
| | 3. Stop running senseid: `pkill -x senseid \|\| true` |
| | 4. Swap symlink: `ln -sf <daemon>/target/debug/senseid ~/.local/bin/senseid` |
| | 5. Launch binary: `Sensei.app/Contents/MacOS/sensei-desktop` with env vars |
| | 6. Wait for `/tmp/tauri-playwright.sock` (up to 60 s) |
| globalTeardown | 1. Kill Sensei.app by saved PID |
| | 2. Stop senseid: `pkill -x senseid \|\| true` |
| | 3. Restore symlink to release binary: `ln -sf <daemon>/target/release/senseid ~/.local/bin/senseid` |

PID is written to `/tmp/sensei-e2e-pid` by globalSetup and read by globalTeardown.

## File Map

### New / replaced files

| File | Action | Purpose |
|------|--------|---------|
| `e2e/globalSetup.ts` | Create | Build both binaries, swap symlink, launch app, wait for socket |
| `e2e/globalTeardown.ts` | Create | Kill app, stop senseid, restore symlink to release |
| `e2e/fixtures.ts` | Replace | Socket-only fixture — no `ipcMocks`, no `devUrl`, no `browser` project logic |
| `e2e/playwright.config.ts` | Replace | Single `tauri` project, `globalSetup`/`globalTeardown`, remove `browser` project + `webServer` |
| `e2e/tests/boot-flow.spec.ts` | Replace | Real bootstrap: gates visible, no crash, advances to setup when healthy |
| `e2e/tests/setup-wizard.spec.ts` | Replace | Same UI assertions, remove mock fixture imports |
| `e2e/tests/db-setup.spec.ts` | Replace | Real DB state: gate reaches `ready` or `blocked`, page handles both |

### Deleted files

| File | Reason |
|------|--------|
| `e2e/fixtures-db-missing.ts` | Stateful mock — not needed with real app |
| `e2e/setup/wizard.spec.ts` | Old daemon-driven test — superseded by rewritten suite |

## Component Details

### `e2e/globalSetup.ts`

```typescript
// Pseudo-code — exact paths:
const DAEMON_REPO  = resolve(__dirname, '../../daemon');
const APP_REPO     = resolve(__dirname, '..');
const SENSEID_DEBUG   = join(DAEMON_REPO, 'target/debug/senseid');
const SENSEID_RELEASE = join(DAEMON_REPO, 'target/release/senseid');
const APP_BINARY   = join(APP_REPO, 'src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop');
const SOCKET       = '/tmp/tauri-playwright.sock';
const PID_FILE     = '/tmp/sensei-e2e-pid';

// 1. cargo build -p senseid (daemon repo)
// 2. cargo tauri build --debug --features e2e-testing (app repo, cwd: src-tauri)
// 3. pkill -x senseid || true; sleep 500ms
// 4. ln -sf SENSEID_DEBUG ~/.local/bin/senseid
// 5. spawn APP_BINARY with SENSEI_MODE=dev, SENSEI_DB_NAME=sensei-dev; write PID
// 6. poll SOCKET existence every 500ms, timeout 60s
```

Build commands are run with `execSync` (blocking — tests must not start before builds finish). Tauri build cwd is `app/src-tauri` because `tauri build` needs to run from there.

### `e2e/globalTeardown.ts`

```typescript
// 1. read PID from PID_FILE; kill process
// 2. pkill -x senseid || true
// 3. ln -sf SENSEID_RELEASE ~/.local/bin/senseid
// 4. rm PID_FILE
```

### `e2e/fixtures.ts`

```typescript
import { createTauriTest } from '@srsholmes/tauri-playwright';

export const { test, expect } = createTauriTest({
  mcpSocket: '/tmp/tauri-playwright.sock',
});
```

No `ipcMocks`, no `devUrl`, no `tauriCommand` — the app is already running from globalSetup.

### `e2e/playwright.config.ts`

```typescript
export default defineConfig({
  testDir: './tests',
  timeout: 60_000,       // longer — real app startup per test nav
  retries: 0,
  workers: 1,            // WKWebView shares one window
  globalSetup:    './globalSetup.ts',
  globalTeardown: './globalTeardown.ts',
  projects: [
    { name: 'tauri', use: { mode: 'tauri' } },
  ],
  // No webServer block — Vite is not used
});
```

## Test Content

### `boot-flow.spec.ts`

Tests the `/health` bootstrap page with a real running app:

- `health page loads` — navigates to `/health`, asserts `.bootstrap-page` is visible
- `bootstrap gates are visible` — at least one `.gate-row` rendered
- `page advances to setup when bootstrap completes` — if all gates become ready within 30 s, page navigates to `/setup/welcome` or `/observatory`; otherwise stays on `/health` — either outcome is a pass (environment-dependent)

### `setup-wizard.spec.ts`

Same assertions as the current file. Remove `import { test, expect } from '../fixtures'` (already from fixtures). No changes to actual test logic — these are pure UI assertions that work identically with real IPC.

### `db-setup.spec.ts`

- `database gate reaches a terminal state` — navigate to `/health`, wait up to 20 s for the `.status-pill` inside the database `.gate-row` to show `ready` or `blocked` (not `waiting` or `checking`)
- `page does not crash when DB is missing` — if gate is `blocked`, the remedy UI (`.remedy`) is visible

## npm scripts

Add to `package.json`:

```json
"test:e2e:desktop": "playwright test --config e2e/playwright.config.ts --project=tauri"
```

The existing `test:e2e` script pointed at the old `tauri` project — update it to use the same command or alias it.

## Constraints

- Both `cargo build` steps run synchronously in globalSetup — first run is slow (minutes); subsequent runs are incremental (seconds).
- Tests require PostgreSQL running locally (bootstrap gate check depends on it).
- The `navigateTo` helper (SvelteKit `goto()` via `evaluate`) must still be used — WKWebView does not tolerate `page.goto()` reloads.
- `workers: 1` is mandatory — Tauri has one WebView window; parallel test workers would race on the same UI.
