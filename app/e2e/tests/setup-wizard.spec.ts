/**
 * Setup Wizard E2E — user-journey flows.
 *
 * Tests mimic real user behaviour: start at /setup/welcome, navigate only via
 * button clicks and form interactions, never by injecting URLs mid-flow.
 *
 * URL monitoring is active throughout each flow — any unexpected redirect to
 * /health is a hard failure (this catches the kind of flash bug that URL
 * injection hides entirely).
 *
 * Selectors are the app's stable semantic hooks (data-action / data-testid /
 * data-component / data-rail-item) or stable copy — never utility classes — so
 * they survive styling churn and are shared with the inspect harness.
 *
 * The wizard is 5 stages: welcome → assistants → roots → scan → done.
 *
 * Two flows:
 *   Flow A — Empty corpus (/tmp/sensei-e2e-empty): a folder with no git repos.
 *   Flow B — Real corpus (/tmp/sensei-e2e-corpus): a minimal git repo.
 *
 * Health gate: seeded via sessionStorage before each flow, matching the state
 * a real user has after passing the health screen once in the same session.
 */

import { test, expect } from '../fixtures';
import { navigateTo, navigateToScreen, DAEMON_URL } from '../helpers';
import { execFileSync } from 'child_process';
import { mkdirSync, writeFileSync, existsSync } from 'fs';

// ── Corpus helpers ────────────────────────────────────────────────────────────

const EMPTY_CORPUS = '/tmp/sensei-e2e-empty';
const REAL_CORPUS  = '/tmp/sensei-e2e-corpus';
const REAL_PROJECT = `${REAL_CORPUS}/sample-app`;

function createEmptyCorpus(): void {
  if (!existsSync(EMPTY_CORPUS)) mkdirSync(EMPTY_CORPUS, { recursive: true });
}

function createRealCorpus(): void {
  if (existsSync(`${REAL_PROJECT}/.git`)) return;
  mkdirSync(`${REAL_PROJECT}/src`, { recursive: true });
  writeFileSync(`${REAL_PROJECT}/package.json`, JSON.stringify({
    name: 'sample-app',
    version: '1.0.0',
    dependencies: { 'lodash': '^4.17.21' },
  }, null, 2));
  writeFileSync(`${REAL_PROJECT}/src/index.ts`,
    `import { cloneDeep } from 'lodash';\nexport const copy = cloneDeep;\n`);
  const opts = { cwd: REAL_PROJECT, stdio: 'ignore' as const };
  execFileSync('git', ['init'],                                          opts);
  execFileSync('git', ['config', 'user.email', 'test@sensei.test'],     opts);
  execFileSync('git', ['config', 'user.name',  'Sensei Test'],          opts);
  execFileSync('git', ['add', '.'],                                      opts);
  execFileSync('git', ['commit', '-m', 'Initial commit'],                opts);
}

// ── Navigation helpers ────────────────────────────────────────────────────────

async function bodyText(tauriPage: any): Promise<string> {
  return (await tauriPage.evaluate(`document.body.textContent ?? ''`)) as string;
}

// `?force=1` is REQUIRED in e2e: the (config) layout's onMount redirects to the
// observatory when `appState.setupOk` (setup already complete) unless the URL
// carries `force=1` — the "re-run setup" escape hatch. globalSetup seeds
// setup-complete so the observatory specs work, so without force the wizard
// bounces to / and the rail never mounts.
const WELCOME = '/setup/welcome?force=1';

async function startAtWelcome(tauriPage: any): Promise<void> {
  // /logs is ALWAYS_REACHABLE — visiting it unmounts the (config) layout so the
  // next navigation remounts fresh (onMount → loadWizardData with the reset
  // daemon).
  await navigateTo(tauriPage, '/logs');
  // /setup/* is gated behind healthOk (hooks.ts::reroute); on a cold e2e DB the
  // health probe takes ~50s to reach 'ok', so RE-navigate until the rail mounts.
  // Mirrors the observatory shell's navigateToScreen.
  await navigateToScreen(tauriPage, WELCOME, '[data-testid="rail"]');
}

async function clickAndExpectNav(
  tauriPage: any,
  selector: string,
  expectedPath: string,
  timeout = 10_000,
): Promise<void> {
  const seen: string[] = [];
  const deadline = Date.now() + timeout;

  await tauriPage.click(selector);

  let reached = false;
  while (Date.now() < deadline) {
    try {
      const p = await tauriPage.evaluate(`window.location.pathname`);
      if (typeof p === 'string') {
        seen.push(p);
        if (p === expectedPath) { reached = true; break; }
      }
    } catch { /* mid-transition */ }
    await new Promise<void>(r => setTimeout(r, 80));
  }

  const unexpected = seen.filter(p => p === '/health');
  expect(unexpected, `Unexpected redirect to /health while navigating to ${expectedPath}`).toHaveLength(0);

  if (!reached) {
    const current = await tauriPage.evaluate(`window.location.pathname`).catch(() => '(unknown)');
    throw new Error(`Timed out (${timeout}ms) waiting for ${expectedPath}. Current: ${current}`);
  }
}

/** Clear any roots accumulated from previous runs (idempotent). */
async function clearRoots(tauriPage: any): Promise<void> {
  const removes = tauriPage.locator('[data-testid="root-remove"]');
  // Remove one at a time, waiting for EACH removal (api + state update) to land
  // before the next click — clicking all at once races the async removals and
  // leaves a residual root, which then fails a "no roots" assertion.
  for (let n = await removes.count(); n > 0; n--) {
    await removes.first().click();
    await expect(removes).toHaveCount(n - 1, { timeout: 10_000 });
  }
}

/** Drive the wizard from welcome to the scan page with the given corpus path.
 *  The scan stage auto-starts on mount, so no "Begin scan" click is needed. */
async function driveToScan(tauriPage: any, corpusPath: string): Promise<void> {
  await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
  await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/roots');
  await tauriPage.locator('.folder-input').fill(corpusPath);
  await tauriPage.click('.btn-solid'); // Add folder
  // Adding a root is async; Continue gates on roots.length > 0. Wait for it to
  // enable before clicking, or the click lands while disabled and we're stuck.
  await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 10_000 });
  await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/scan');
}

// ── Flow A: Empty corpus ──────────────────────────────────────────────────────

test.describe('Setup Wizard — Flow A: empty corpus', () => {
  // /setup/* is health-gated; a cold e2e boot takes ~50s, longer than the
  // default 60s per-test timeout (matches the multi-window gated-route budget).
  test.describe.configure({ timeout: 150_000 });
  test.beforeAll(() => { createEmptyCorpus(); });

  test.beforeEach(async ({ tauriPage }) => {
    try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
    await startAtWelcome(tauriPage);
  });

  // ── Welcome ─────────────────────────────────────────────────────────────
  test('welcome: hero text, three pillars, Continue enabled', async ({ tauriPage }) => {
    const text = await bodyText(tauriPage);
    expect(text).toContain('A teacher does not');
    expect(text).toContain('write the code');
    for (const pillar of ['Observe', 'Teach', 'Local']) expect(text).toContain(pillar);
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled();
  });

  test('welcome → assistants: Continue navigates, no health flash', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
  });

  // ── Assistants ───────────────────────────────────────────────────────────
  test('assistants: cards render or empty state, Continue always enabled', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');

    const cards = tauriPage.locator('[data-testid^="assistant-card-"]');
    // The SSE-driven list is async — wait until it settles into ONE of the two
    // honest outcomes (a card is present, or the explicit empty message), then
    // assert that outcome specifically.
    await expect
      .poll(
        async () =>
          (await cards.count()) > 0 ||
          (await bodyText(tauriPage)).includes('No AI coding assistants detected'),
        { timeout: 8_000 },
      )
      .toBe(true);
    if ((await cards.count()) === 0) {
      expect(await bodyText(tauriPage)).toContain('No AI coding assistants detected');
    } else {
      await expect(cards.first()).toBeVisible();
    }
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled();
  });

  test('assistants → roots: navigates, no health flash', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/roots');
  });

  // ── Roots ────────────────────────────────────────────────────────────────
  test('roots: gate — disabled with no roots, enabled after adding one', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/roots');
    await clearRoots(tauriPage);

    await expect(tauriPage.locator('[data-action="next"]')).toBeDisabled();
    await tauriPage.locator('.folder-input').fill(EMPTY_CORPUS);
    await tauriPage.click('.btn-solid');
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled();
  });

  test('roots: Enter key adds folder, duplicate is rejected', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/roots');
    await clearRoots(tauriPage);

    const rootPath = tauriPage.locator('[data-component="root-path"]').filter({ hasText: EMPTY_CORPUS });
    await tauriPage.locator('.folder-input').fill(EMPTY_CORPUS);
    await tauriPage.locator('.folder-input').press('Enter');
    await expect(rootPath).toBeVisible();
    await tauriPage.locator('.folder-input').fill(EMPTY_CORPUS);
    await tauriPage.click('.btn-solid');
    await expect(rootPath).toHaveCount(1);
  });

  // ── Scan (empty corpus) ──────────────────────────────────────────────────
  test('scan: auto-starts → stats + tasks panel, Continue enables when idle', async ({ tauriPage }) => {
    await driveToScan(tauriPage, EMPTY_CORPUS);

    // Scan engages on mount — the started view (stats bar + tasks panel) renders
    // without a manual Begin-scan click.
    await expect(tauriPage.locator('[data-testid="scan-tasks-panel"]')).toBeVisible({ timeout: 8_000 });
    expect(await bodyText(tauriPage)).toContain('ROOTS');

    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 20_000 });
  });

  // ── Scan → Done ──────────────────────────────────────────────────────────
  test('scan → done: Continue reaches the observatory-entry ceremony', async ({ tauriPage }) => {
    await driveToScan(tauriPage, EMPTY_CORPUS);
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 20_000 });

    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/done');
    await expect(tauriPage.locator('[data-testid="done-summary"]')).toBeVisible();
    await expect(tauriPage.locator('[data-action="next"]')).toContainText('Enter observatory');
  });
});

// ── Flow B: Real corpus ───────────────────────────────────────────────────────

test.describe('Setup Wizard — Flow B: real corpus', () => {
  test.describe.configure({ timeout: 150_000 });
  test.beforeAll(() => { createRealCorpus(); });

  test.beforeEach(async ({ tauriPage }) => {
    try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
    await startAtWelcome(tauriPage);
  });

  test('scan: real corpus scan starts, stats visible, Continue enables when idle', async ({ tauriPage }) => {
    await driveToScan(tauriPage, REAL_CORPUS);
    await expect(tauriPage.locator('[data-testid="scan-tasks-panel"]')).toBeVisible({ timeout: 8_000 });
    expect(await bodyText(tauriPage)).toContain('ROOTS');
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 60_000 });
  });
});

// ── Rail structure (fast standalone checks) ───────────────────────────────────

test.describe('Setup Wizard — Rail', () => {
  test.describe.configure({ timeout: 150_000 });
  test.beforeEach(async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, WELCOME, '[data-testid="rail"]');
  });

  // The rail iterates every entry in STAGES (5 stages).
  test('shows 5 stages in the rail', async ({ tauriPage }) => {
    await expect(tauriPage.locator('[data-rail-item]')).toHaveCount(5);
  });

  test('welcome stage is active on load', async ({ tauriPage }) => {
    await expect(tauriPage.locator('[data-rail-item][data-active="true"]')).toContainText('Welcome');
  });

  test('active stage advances after Continue click', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
    await expect(tauriPage.locator('[data-rail-item][data-active="true"]')).toContainText('Assistants');
  });
});
