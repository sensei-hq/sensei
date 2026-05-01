/**
 * Setup Wizard E2E tests.
 *
 * In browser mode: mocked IPC, tests the UI flow.
 * In Tauri mode: real daemon, tests the full stack.
 */

import { test, expect } from '../fixtures';

test.describe('Setup Wizard — Welcome', () => {
  test('renders welcome page with hero text', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/welcome');
    await expect(tauriPage.locator('.hero')).toContainText('A teacher does not');
    await expect(tauriPage.locator('.hero-accent')).toContainText('write the code');
  });

  test('shows three pillars: Observe, Teach, Local', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/welcome');
    await expect(tauriPage.locator('.pillar-title').nth(0)).toContainText('Observe');
    await expect(tauriPage.locator('.pillar-title').nth(1)).toContainText('Teach');
    await expect(tauriPage.locator('.pillar-title').nth(2)).toContainText('Local');
  });

  test('Continue button is enabled on welcome', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/welcome');
    const btn = tauriPage.locator('.btn-primary');
    await expect(btn).toBeEnabled();
    await expect(btn).toContainText('Continue');
  });

  test('clicking Continue advances to Preferences', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/welcome');
    await tauriPage.click('.btn-primary');
    await tauriPage.waitForURL('**/setup/preferences');
  });
});

test.describe('Setup Wizard — Rail navigation', () => {
  test('rail shows 11 stages', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/welcome');
    const items = tauriPage.locator('.rail-item');
    await expect(items).toHaveCount(11);
  });

  test('first stage is active on welcome', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/welcome');
    const active = tauriPage.locator('.rail-item.active');
    await expect(active).toContainText('Welcome');
  });
});

test.describe('Setup Wizard — Preferences gate', () => {
  test('Continue is disabled on preferences (empty displayName)', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/preferences');
    const btn = tauriPage.locator('.btn-primary');
    await expect(btn).toBeDisabled();
  });
});
