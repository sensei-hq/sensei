/**
 * Settings rail — sub-routes exist and render.
 *
 * After the wizard → Preferences arch change, /settings is a rail with three
 * groups (You / Sources / Reasoning) plus an Extensions leaf. Each entry
 * navigates to a real route with hydrated content.
 *
 * This spec walks every rail entry and asserts a page-scoped landmark is
 * present after navigation. It intentionally uses `navigateTo` (direct URL)
 * rather than clicking rail links — rokkit's `List` link click behaviour is
 * covered by the vitest sidebar spec; this file is about the routes.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

test.describe('Settings — Rail sub-routes', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedHealth(tauriPage);
  });

  test('/settings redirects to /settings/general', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings');
    const path = await tauriPage.evaluate(`window.location.pathname`);
    expect(path).toBe('/settings/general');
    await expect(tauriPage.locator('[data-testid="settings-general"]')).toBeVisible({ timeout: 10_000 });
  });

  test('/settings/general renders the Preferences form', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/general');
    await expect(tauriPage.locator('[data-testid="pref-display-name"]')).toBeVisible({ timeout: 10_000 });
    await expect(tauriPage.locator('[data-testid="pref-digest-cadence"]')).toBeVisible();
    await expect(tauriPage.locator('[data-testid="pref-correction-aggressiveness"]')).toBeVisible();
  });

  test('/settings/assistants renders (assistants list or empty state)', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/assistants');
    // Page mount is enough — assert the content wrapper (a stable data-testid; the
    // `text=/…/` pseudo-locator isn't supported by the socket harness).
    await expect(tauriPage.locator('[data-testid="settings-assistants"]')).toBeVisible({ timeout: 10_000 });
  });

  test('/settings/roots renders the input + list', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/roots');
    await expect(tauriPage.locator('.folder-input').first()).toBeVisible({ timeout: 10_000 });
  });

  test('/settings/projects renders', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/projects');
    await expect(tauriPage.locator('[data-testid="settings-projects"]')).toBeVisible({ timeout: 10_000 });
  });

  test('/settings/libraries renders (summary or empty state)', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/libraries');
    await expect(
      tauriPage.locator('[data-testid="libraries-empty"], [data-testid="libraries-summary"]').first()
    ).toBeVisible({ timeout: 10_000 });
  });

  test('/settings/instruments renders', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/instruments');
    // Either the empty-state placeholder or an MCP card lands.
    await expect(
      tauriPage.locator('[data-testid="instruments-empty"], [data-testid^="mcp-card-"]').first()
    ).toBeVisible({ timeout: 10_000 });
  });

  test('/settings/providers renders router cards', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/providers');
    await expect(tauriPage.locator('[data-testid^="router-card-"]').first()).toBeVisible({ timeout: 10_000 });
  });

  test('/settings/inference renders the role → chain picker', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/inference');
    await expect(tauriPage.locator('[data-testid="inference-role-picker-inference"]')).toBeVisible({
      timeout: 10_000,
    });
  });

  test('/settings/extensions renders', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings/extensions');
    await expect(tauriPage.locator('[data-testid="settings-extensions"]')).toBeVisible({ timeout: 10_000 });
  });
});
