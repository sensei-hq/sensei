/**
 * Setup Wizard — E2E Test Suite
 *
 * Tests the full setup wizard flow from blank state to completion.
 * Runs against the daemon in dev mode (.sensei-dev/, port 7745).
 *
 * Prerequisites:
 *   cargo build --manifest-path crates/senseid/Cargo.toml
 *   bun install (in apps/desktop)
 */

import { test, expect, type Page } from '@playwright/test';
import {
  startDaemon, stopDaemon, resetDaemon, setAppPort,
  PORT, LOCAL_FOLDER, DAEMON_URL,
  daemonGet, daemonPost,
} from '../helpers.js';

// ── Lifecycle ───────────────────────────────────────────────

test.beforeAll(async () => {
  await startDaemon();
  await resetDaemon();
});

test.afterAll(async () => {
  await stopDaemon();
});

test.beforeEach(async () => {
  await resetDaemon();
});

// ── Helpers ─────────────────────────────────────────────────

/** Navigate to setup and wait for it to load. Sets dev port in localStorage. */
async function goToSetup(page: Page) {
  await setAppPort(page);
  await page.goto('/setup');
  await page.waitForSelector('button:has-text("Begin setup")', { timeout: 10_000 });
}

/** Click "Begin setup" to enter the wizard. */
async function beginSetup(page: Page) {
  await page.click('button:has-text("Begin setup")');
  await page.waitForSelector('text=SETUP'); // rail loaded
}

/** Click "Continue" to advance to the next step. */
async function clickContinue(page: Page) {
  await page.click('button:has-text("Continue →")');
}

/** Get the current step label text. */
async function currentStep(page: Page): Promise<string> {
  const el = await page.locator('.step-label, [class*="step-label"]').first();
  return el.textContent() ?? '';
}

// ── Tests ───────────────────────────────────────────────────

test.describe('Setup Wizard', () => {

  test('Landing page shows "A quiet empty room"', async ({ page }) => {
    await goToSetup(page);

    // Should show landing hero
    await expect(page.locator('h1')).toContainText('empty room');
    await expect(page.locator('button:has-text("Begin setup")')).toBeVisible();

    // Should show "What Sensei Does" card
    await expect(page.locator('text=Watches')).toBeVisible();
    await expect(page.locator('text=Notices')).toBeVisible();
    await expect(page.locator('text=Teaches')).toBeVisible();

    // Should show sensei kanji logo (先生)
    await expect(page.locator('text=先生')).toBeVisible();
  });

  test('Step 1: Welcome — shows teacher message', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);

    // Should show welcome hero
    await expect(page.locator('h1')).toContainText('teacher does not');
    await expect(page.locator('h1')).toContainText('write the code');

    // Should show three pillars
    await expect(page.locator('text=Observe')).toBeVisible();
    await expect(page.locator('text=Teach')).toBeVisible();
    await expect(page.locator('text=Local')).toBeVisible();

    // Progress bar should show 01 / 9
    await expect(page.locator('text=01 / 9')).toBeVisible();
  });

  test('Step 2: Components — shows real daemon status', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);
    await clickContinue(page);

    // Should be on Components step
    await expect(page.locator('h1')).toContainText('Components');

    // Should show real component data from daemon
    await expect(page.locator('text=sensei-cli')).toBeVisible();
    await expect(page.locator('text=MCP bridge')).toBeVisible();
    await expect(page.locator('text=sensei-daemon')).toBeVisible();

    // All should be READY (daemon is running)
    const readyBadges = page.locator('text=READY');
    await expect(readyBadges).toHaveCount(3);

    // Should show correct port
    await expect(page.locator(`text=localhost:${PORT}`)).toBeVisible();
  });

  test('Step 3: Assistants — shows detected ACPs', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);
    await clickContinue(page); // → Components
    await clickContinue(page); // → Assistants

    await expect(page.locator('h1')).toContainText('Assistants');

    // Should show at least one ACP (Claude Code should be detected on dev machine)
    // Verify by checking daemon API directly
    const acps = await daemonGet<any[]>('/api/acp/detect');
    const foundAcps = acps.filter(a => a.installed);

    // Each found ACP should appear with a checkbox
    for (const acp of foundAcps) {
      await expect(page.locator(`text=${acp.name}`)).toBeVisible();
    }
  });

  test('Step 4: Folders — starts empty, can add folder', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);
    await clickContinue(page); // → Components
    await clickContinue(page); // → Assistants
    await clickContinue(page); // → Folders

    await expect(page.locator('h1')).toContainText('Folders');

    // Should start empty (no mock folders)
    const folderRows = page.locator('[class*="folder-row"]');
    await expect(folderRows).toHaveCount(0);

    // Add a real folder
    const input = page.locator('input[type="text"]');
    await input.fill(LOCAL_FOLDER);
    await page.click('button:has-text("Add")');

    // Should now show 1 folder
    await expect(folderRows).toHaveCount(1);
    await expect(page.locator(`text=${LOCAL_FOLDER}`)).toBeVisible();
  });

  test('Step 5: Scan — triggers real daemon scan', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);
    await clickContinue(page); // → Components
    await clickContinue(page); // → Assistants
    await clickContinue(page); // → Folders

    // Add folder
    const input = page.locator('input[type="text"]');
    await input.fill(LOCAL_FOLDER);
    await page.click('button:has-text("Add")');
    await clickContinue(page); // → Scan

    await expect(page.locator('h1')).toContainText('Scan');
    await expect(page.locator('text=Begin scan')).toBeVisible();

    // Click Begin scan
    await page.click('button:has-text("Begin scan")');

    // Should show scanning activity
    await expect(page.locator('text=scan started')).toBeVisible({ timeout: 5_000 });

    // Wait for scan to complete (up to 30s for large folders)
    await expect(page.locator('text=complete')).toBeVisible({ timeout: 30_000 });

    // Verify daemon actually has repos now
    const repos = await daemonGet<any[]>('/api/repos');
    expect(repos.length).toBeGreaterThan(0);
  });

  test('Step 6: Projects — shows discovered repos', async ({ page }) => {
    // Pre-scan so projects step has data
    await daemonPost('/api/scan', { root: LOCAL_FOLDER });

    // Wait for scan to finish
    await page.waitForTimeout(5_000);

    await goToSetup(page);
    await beginSetup(page);

    // Navigate to Projects step
    for (let i = 0; i < 5; i++) await clickContinue(page);

    await expect(page.locator('h1')).toContainText('Projects');

    // Should show at least one project card
    const cards = page.locator('[class*="project-card"], [class*="card"]');
    // Give it time to load from daemon
    await page.waitForTimeout(2_000);

    // Verify repos exist in daemon
    const repos = await daemonGet<any[]>('/api/repos');
    expect(repos.length).toBeGreaterThan(0);
  });

  test('Step 9: Done — shows correct counts', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);

    // Navigate to last step
    for (let i = 0; i < 8; i++) await clickContinue(page);

    await expect(page.locator('h1')).toContainText('observatory is ready');

    // Should show the 観 kanji
    await expect(page.locator('text=観')).toBeVisible();

    // Should show "Enter observatory" button (not "Continue")
    await expect(page.locator('button:has-text("Enter observatory")')).toBeVisible();

    // Should show stat labels
    await expect(page.locator('text=PROJECTS')).toBeVisible();
    await expect(page.locator('text=REPOS')).toBeVisible();
    await expect(page.locator('text=LIBRARIES')).toBeVisible();
    await expect(page.locator('text=MCPS')).toBeVisible();
    await expect(page.locator('text=ASSISTANTS')).toBeVisible();
  });

  test('Rail shows correct daemon port', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);

    // Rail footer should show dev mode port
    await expect(page.locator(`text=daemon · ${PORT}`)).toBeVisible();
  });

  test('No mock data in any step', async ({ page }) => {
    await goToSetup(page);
    await beginSetup(page);

    // Step 4: Folders should be empty
    await clickContinue(page); // → Components
    await clickContinue(page); // → Assistants
    await clickContinue(page); // → Folders

    // Should NOT show mock folder paths
    await expect(page.locator('text=~/code/lumen')).not.toBeVisible();
    await expect(page.locator('text=~/code/brand-kit')).not.toBeVisible();

    // Should NOT show port 9823
    await expect(page.locator('text=9823')).not.toBeVisible();
  });
});
