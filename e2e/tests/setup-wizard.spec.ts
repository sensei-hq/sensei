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
  test('Continue is disabled when displayName is empty', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    const btn = tauriPage.locator('.btn-primary');
    await expect(btn).toBeDisabled();
  });

  test('renders stage header with correct title', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    await expect(tauriPage.locator('.stage-title')).toContainText('Preferences');
  });

  test('renders all four sections', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    const sections = tauriPage.locator('.section');
    await expect(sections).toHaveCount(4);
  });

  test('name input is present and editable', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    const input = tauriPage.locator('.name-input');
    await expect(input).toBeVisible();
    await input.fill('Keiko');
    await expect(input).toHaveValue('Keiko');
  });

  test('Continue enables after typing a name', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    const btn = tauriPage.locator('.btn-primary');
    await expect(btn).toBeDisabled();
    await tauriPage.locator('.name-input').fill('Jerry');
    await expect(btn).toBeEnabled();
  });

  test('clicking Continue after entering name advances to Assistants', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    await tauriPage.locator('.name-input').fill('Jerry');
    await tauriPage.click('.btn-primary');
    await tauriPage.waitForURL('/setup/assistants');
  });

  test('toggles work for shared learnings', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    const toggle = tauriPage.locator('[aria-label="Toggle contribute learnings"]');
    await expect(toggle).toBeVisible();
    await toggle.click();
  });

  test('segment control works for correction tone', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    // :text() is Playwright-only — TauriPage uses document.querySelector which
    // only supports standard CSS. Find and click by text content via evaluate.
    await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('.segment-btn'))
        .find(b => b.textContent.trim() === 'Gentle')?.click()
    `);
    await expect(tauriPage.locator('.segment-btn.active').first()).toContainText('Gentle');
  });

  test('select works for sharing schedule', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    const sel = tauriPage.locator('.sel').first();
    await sel.selectOption('daily');
    await expect(sel).toHaveValue('daily');
  });
});
