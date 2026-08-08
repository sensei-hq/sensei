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
 * The wizard is 5 stages after the wizard → Preferences arch change:
 *   welcome → assistants → roots → scan → done
 *
 * Everything else (Preferences, Projects, Libraries, Instruments, Providers,
 * Inference, Assignments) lives in Settings and has its own coverage.
 *
 * Two flows:
 *   Flow A — Empty corpus (/tmp/sensei-e2e-empty): a folder with no git repos.
 *            Scan completes instantly.
 *
 *   Flow B — Real corpus (/tmp/sensei-e2e-corpus): a minimal git repo with a
 *            package.json.
 *
 * Health gate: seeded via sessionStorage before each flow, matching the state
 * a real user has after passing the health screen once in the same session.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';
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

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

async function startAtWelcome(tauriPage: any): Promise<void> {
  await seedHealth(tauriPage);
  // /logs is HEALTH_EXEMPT — always reachable regardless of gate state.
  // Visiting it unmounts the (config) layout so the next navigation remounts
  // it fresh, triggering onMount → loadWizardData with the reset daemon.
  await navigateTo(tauriPage, '/logs');
  await navigateTo(tauriPage, '/setup/welcome');
  await expect(tauriPage.locator('[data-testid="rail"]')).toBeVisible({ timeout: 12_000 });
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

/** Drive the wizard from welcome to the scan page with the given corpus path. */
async function driveToScan(tauriPage: any, corpusPath: string): Promise<void> {
  await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
  await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/roots');
  await tauriPage.locator('.folder-input').fill(corpusPath);
  await tauriPage.click('.btn-solid'); // Add folder
  await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/scan');
}

// ── Flow A: Empty corpus ──────────────────────────────────────────────────────

test.describe('Setup Wizard — Flow A: empty corpus', () => {
  test.beforeAll(() => { createEmptyCorpus(); });

  test.beforeEach(async ({ tauriPage }) => {
    try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
    await startAtWelcome(tauriPage);
  });

  // ── Welcome ─────────────────────────────────────────────────────────────
  test('welcome: hero text, three pillars, Continue enabled', async ({ tauriPage }) => {
    await expect(tauriPage.locator('.hero')).toContainText('A teacher does not');
    await expect(tauriPage.locator('.hero-accent')).toContainText('write the code');
    await expect(tauriPage.locator('.pillar-title').nth(0)).toContainText('Observe');
    await expect(tauriPage.locator('.pillar-title').nth(1)).toContainText('Teach');
    await expect(tauriPage.locator('.pillar-title').nth(2)).toContainText('Local');
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled();
  });

  test('welcome → assistants: Continue navigates, no health flash', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
  });

  // ── Assistants ───────────────────────────────────────────────────────────
  test('assistants: cards render or empty state, Continue always enabled', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
    await expect(tauriPage.locator('.assistants')).toBeVisible({ timeout: 8_000 });

    const cardCount = await tauriPage.locator('.card').count();
    if (cardCount > 0) {
      const names = await tauriPage.evaluate(
        `Array.from(document.querySelectorAll('.card-name')).map(el => el.textContent?.trim() ?? '')`
      ) as string[];
      for (const name of names) expect(name.length).toBeGreaterThan(0);
    } else {
      await expect(tauriPage.locator('.empty')).toBeVisible();
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

    // Clear any roots accumulated from previous test runs.
    const removes = tauriPage.locator('.btn-remove');
    for (let i = await removes.count(); i > 0; i--) {
      await removes.first().click();
    }

    await expect(tauriPage.locator('[data-action="next"]')).toBeDisabled();
    await tauriPage.locator('.folder-input').fill(EMPTY_CORPUS);
    await tauriPage.click('.btn-solid');
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled();
  });

  test('roots: Enter key adds folder, duplicate is rejected', async ({ tauriPage }) => {
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/assistants');
    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/roots');

    const removes = tauriPage.locator('.btn-remove');
    for (let i = await removes.count(); i > 0; i--) {
      await removes.first().click();
    }

    await tauriPage.locator('.folder-input').fill(EMPTY_CORPUS);
    await tauriPage.locator('.folder-input').press('Enter');
    await expect(tauriPage.locator('.folder-path').filter({ hasText: EMPTY_CORPUS })).toBeVisible();
    await tauriPage.locator('.folder-input').fill(EMPTY_CORPUS);
    await tauriPage.click('.btn-solid');
    await expect(tauriPage.locator('.folder-path').filter({ hasText: EMPTY_CORPUS })).toHaveCount(1);
  });

  // ── Scan (empty corpus) ──────────────────────────────────────────────────
  test('scan: Begin scan → stats bar, Continue disabled then enabled when idle', async ({ tauriPage }) => {
    await driveToScan(tauriPage, EMPTY_CORPUS);

    await expect(tauriPage.locator('[data-action="next"]')).toBeDisabled();
    await expect(tauriPage.locator('.hero-card')).toBeVisible();

    await tauriPage.click('.btn-solid'); // Begin scan
    await expect(tauriPage.locator('.stats-bar')).toBeVisible({ timeout: 5_000 });
    await expect(tauriPage.locator('.hero-card')).not.toBeVisible();
    await expect(tauriPage.locator('.stat-label').nth(0)).toContainText('ROOTS');

    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 20_000 });
  });

  // ── Scan → Done ──────────────────────────────────────────────────────────
  test('scan → done: Continue reaches the observatory-entry ceremony', async ({ tauriPage }) => {
    await driveToScan(tauriPage, EMPTY_CORPUS);
    await tauriPage.click('.btn-solid');
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 20_000 });

    await clickAndExpectNav(tauriPage, '[data-action="next"]', '/setup/done');
    await expect(tauriPage.locator('[data-testid="done-summary"]')).toBeVisible();
    await expect(tauriPage.locator('[data-action="next"]')).toContainText('Enter observatory');
  });
});

// ── Flow B: Real corpus ───────────────────────────────────────────────────────

test.describe('Setup Wizard — Flow B: real corpus', () => {
  test.beforeAll(() => { createRealCorpus(); });

  test.beforeEach(async ({ tauriPage }) => {
    try { await fetch(`${DAEMON_URL}/api/reset`, { method: 'POST' }); } catch { /* ok */ }
    await startAtWelcome(tauriPage);
  });

  test('scan: real corpus scan starts, stats visible, Continue enables when idle', async ({ tauriPage }) => {
    await driveToScan(tauriPage, REAL_CORPUS);
    await tauriPage.click('.btn-solid');
    await expect(tauriPage.locator('.stats-bar')).toBeVisible({ timeout: 5_000 });
    await expect(tauriPage.locator('.stat-label').nth(0)).toContainText('ROOTS');
    await expect(tauriPage.locator('[data-action="next"]')).toBeEnabled({ timeout: 60_000 });
  });
});

// ── Rail structure (fast standalone checks) ───────────────────────────────────

test.describe('Setup Wizard — Rail', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/setup/welcome');
    await expect(tauriPage.locator('[data-testid="rail"]')).toBeVisible({ timeout: 12_000 });
  });

  // The rail iterates every entry in STAGES (5 after the wizard →
  // Preferences arch change).
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
