/**
 * Setup Wizard — E2E Test Suite
 *
 * Tests the full setup wizard flow from blank state to completion.
 * Runs against the daemon in dev mode (.sensei-dev/, port 7745).
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

// ── Helpers ─────────────────────────────────────────────────

async function goToSetup(page: Page) {
  await setAppPort(page);
  await page.goto('/setup', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1500); // let Svelte hydrate + daemon fetch
}

async function beginSetup(page: Page) {
  await page.click('button:has-text("Begin setup")');
  await page.waitForTimeout(1000);
}

async function clickContinue(page: Page) {
  await page.click('button:has-text("Continue →")');
  await page.waitForTimeout(800);
}

async function navigateToStep(page: Page, stepNumber: number) {
  await goToSetup(page);
  await beginSetup(page);
  for (let i = 1; i < stepNumber; i++) {
    await clickContinue(page);
  }
}

// ── Tests ───────────────────────────────────────────────────

test.describe('Setup Wizard', () => {

  // Reset only once at start — not between tests to avoid killing mid-scan

  test('Landing page shows "A quiet empty room"', async ({ page }) => {
    await goToSetup(page);

    await expect(page.locator('h1')).toContainText('empty room');
    await expect(page.locator('button:has-text("Begin setup")')).toBeVisible();

    // Info card content — use exact match to avoid substring collisions
    await expect(page.getByText('Watches', { exact: true })).toBeVisible();
    await expect(page.getByText('Notices', { exact: true })).toBeVisible();
    await expect(page.getByText('Teaches', { exact: true })).toBeVisible();

    // Logo kanji
    await expect(page.getByText('先生').first()).toBeVisible();
  });

  test('Step 1: Welcome', async ({ page }) => {
    await navigateToStep(page, 1);

    await expect(page.locator('h1')).toContainText('teacher does not');

    // Three pillars
    await expect(page.getByText('Observe', { exact: true })).toBeVisible();
    await expect(page.getByText('Teach', { exact: true })).toBeVisible();
    await expect(page.getByText('Local', { exact: true })).toBeVisible();

    await expect(page.getByText('01 / 9')).toBeVisible();
  });

  test('Step 2: Components — real daemon data', async ({ page }) => {
    await navigateToStep(page, 2);

    await expect(page.locator('h1')).toContainText('Components');
    await expect(page.getByText('sensei-cli')).toBeVisible();
    await expect(page.getByText('MCP bridge')).toBeVisible();
    await expect(page.getByText('sensei-daemon')).toBeVisible();

    // Count READY badges within the component cards only
    const cards = page.locator('.card');
    await expect(cards).toHaveCount(3);

    // Correct port
    await expect(page.getByText(`localhost:${PORT}`)).toBeVisible();
  });

  test('Step 3: Assistants — detected ACPs', async ({ page }) => {
    await navigateToStep(page, 3);

    await expect(page.locator('h1')).toContainText('Assistants');

    const acps = await daemonGet<any[]>('/api/acp/detect');
    const foundAcps = acps.filter((a: any) => a.installed);
    expect(foundAcps.length).toBeGreaterThan(0);

    // At least one found ACP should be visible
    await expect(page.getByText(foundAcps[0].name)).toBeVisible();
  });

  test('Step 4: Folders — starts empty, can add', async ({ page }) => {
    await navigateToStep(page, 4);

    await expect(page.locator('h1')).toContainText('Folders');

    // No mock folders
    await expect(page.getByText('~/code/lumen')).not.toBeVisible();

    // Add real folder
    await page.locator('input[type="text"]').fill(LOCAL_FOLDER);
    await page.click('button:has-text("Add")');
    await page.waitForTimeout(500);

    await expect(page.getByText(LOCAL_FOLDER)).toBeVisible();
  });

  test('Step 5: Scan — real daemon scan', async ({ page }) => {
    test.setTimeout(120_000);

    await navigateToStep(page, 4);
    await page.locator('input[type="text"]').fill(LOCAL_FOLDER);
    await page.click('button:has-text("Add")');
    await clickContinue(page);

    await expect(page.locator('h1')).toContainText('Scan');
    await page.click('button:has-text("Begin scan")');

    // Wait for scan to produce results (up to 60s for large folders)
    await page.waitForTimeout(15_000);

    // Daemon should have repos
    const repos = await daemonGet<any[]>('/api/repos');
    expect(repos.length).toBeGreaterThan(0);
  });

  test('Step 6: Projects — discovered repos', async ({ page }) => {
    test.setTimeout(120_000);

    // Ensure there are repos from previous scan test, or scan now
    let repos = await daemonGet<any[]>('/api/repos');
    if (repos.length === 0) {
      await daemonPost('/api/scan', { root: LOCAL_FOLDER });
      await page.waitForTimeout(15_000);
      repos = await daemonGet<any[]>('/api/repos');
    }
    expect(repos.length).toBeGreaterThan(0);

    await navigateToStep(page, 6);
    await expect(page.locator('h1')).toContainText('Projects');
  });

  test('Step 9: Done — summary', async ({ page }) => {
    await navigateToStep(page, 9);

    await expect(page.locator('.hero-kanji, [class*="hero"]').first()).toBeVisible();
    await expect(page.getByRole('heading', { name: /observatory is ready/i })).toBeVisible();
    await expect(page.locator('button:has-text("Enter observatory")')).toBeVisible();
  });

  test('Rail shows dev port', async ({ page }) => {
    await navigateToStep(page, 1);
    await expect(page.getByText(`daemon · ${PORT}`)).toBeVisible();
  });

  test('No mock data leaks', async ({ page }) => {
    await navigateToStep(page, 4);
    await expect(page.getByText('~/code/lumen')).not.toBeVisible();
    await expect(page.getByText('9823')).not.toBeVisible();
  });
});
