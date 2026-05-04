/**
 * Setup Wizard E2E tests.
 *
 * In browser mode: mocked IPC, tests the UI flow.
 * In Tauri mode: real daemon, tests the full stack.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

test.describe('Setup Wizard — Welcome', () => {
  test('renders welcome page with hero text', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    await expect(tauriPage.locator('.hero')).toContainText('A teacher does not');
    await expect(tauriPage.locator('.hero-accent')).toContainText('write the code');
  });

  test('shows three pillars: Observe, Teach, Local', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    await expect(tauriPage.locator('.pillar-title').nth(0)).toContainText('Observe');
    await expect(tauriPage.locator('.pillar-title').nth(1)).toContainText('Teach');
    await expect(tauriPage.locator('.pillar-title').nth(2)).toContainText('Local');
  });

  test('Continue button is enabled on welcome', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    const btn = tauriPage.locator('.btn-primary');
    await expect(btn).toBeEnabled();
    await expect(btn).toContainText('Continue');
  });

  test('clicking Continue advances to Preferences', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    await tauriPage.click('.btn-primary');
    await tauriPage.waitForURL('/setup/preferences');
  });
});

test.describe('Setup Wizard — Rail navigation', () => {
  test('rail shows 11 stages', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    const items = tauriPage.locator('.rail-item');
    await expect(items).toHaveCount(11);
  });

  test('first stage is active on welcome', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    const active = tauriPage.locator('.rail-item.active');
    await expect(active).toContainText('Welcome');
  });
});

test.describe('Setup Wizard — Preferences', () => {
  // Direct URL anchor-click to /setup/preferences does not trigger SvelteKit's
  // router within the built Tauri app. Navigate via welcome → Continue instead,
  // which calls goto() internally and lands on preferences correctly.
  test.beforeEach(async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    await expect(tauriPage.locator('.btn-primary')).toBeEnabled({ timeout: 10_000 });
    await tauriPage.click('.btn-primary');
    await tauriPage.waitForURL('/setup/preferences', { timeout: 10_000 });
    // Clear displayName for a clean slate (previous tests or a prior run may have set it)
    await expect(tauriPage.locator('.name-input')).toBeVisible({ timeout: 5_000 });
    await tauriPage.locator('.name-input').fill('');
  });

  test('Continue is disabled when displayName is empty', async ({ tauriPage }) => {
    const btn = tauriPage.locator('.btn-primary');
    await expect(btn).toBeDisabled();
  });

  test('renders stage header with correct title', async ({ tauriPage }) => {
    await expect(tauriPage.locator('.stage-title')).toContainText('Preferences');
  });

  test('renders all four sections', async ({ tauriPage }) => {
    const sections = tauriPage.locator('.section');
    await expect(sections).toHaveCount(4);
  });

  test('name input is present and editable', async ({ tauriPage }) => {
    const input = tauriPage.locator('.name-input');
    await expect(input).toBeVisible();
    await input.fill('Keiko');
    await expect(input).toHaveValue('Keiko');
  });

  test('Continue enables after typing a name', async ({ tauriPage }) => {
    const btn = tauriPage.locator('.btn-primary');
    await expect(btn).toBeDisabled();
    await tauriPage.locator('.name-input').fill('Jerry');
    await expect(btn).toBeEnabled();
  });

  test('clicking Continue after entering name advances to Assistants', async ({ tauriPage }) => {
    await tauriPage.locator('.name-input').fill('Jerry');
    await tauriPage.click('.btn-primary');
    await tauriPage.waitForURL('/setup/assistants');
  });

  test('toggles work for shared learnings', async ({ tauriPage }) => {
    const toggle = tauriPage.locator('[aria-label="Toggle contribute learnings"]');
    await expect(toggle).toBeVisible();
    await toggle.click();
  });

  test('segment control works for correction tone', async ({ tauriPage }) => {
    // TauriPage .first() maps to :nth-match(0) (invalid CSS — 1-indexed).
    // Use evaluate throughout: click by text, verify via querySelector.
    await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('.segment-btn'))
        .find(b => b.textContent.trim() === 'Gentle')?.click()
    `);
    const activeText = await tauriPage.evaluate(
      `document.querySelector('.segment-btn.active')?.textContent?.trim()`
    );
    expect(activeText).toBe('Gentle');
  });

  test('select works for sharing schedule', async ({ tauriPage }) => {
    // Two .sel elements exist (sharing schedule + download collective).
    // Use evaluate to target the first one directly, avoiding .first() / .nth(0)
    // which TauriPage may translate to an invalid :nth-match(0) selector.
    await tauriPage.evaluate(`
      const sel = document.querySelectorAll('.sel')[0];
      sel.value = 'daily';
      sel.dispatchEvent(new Event('change', { bubbles: true }));
    `);
    const val = await tauriPage.evaluate(
      `document.querySelectorAll('.sel')[0].value`
    );
    expect(val).toBe('daily');
  });
});
