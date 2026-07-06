/**
 * Providers surface — router list, paste a key, daemon writes to
 * Keychain. Clear button removes the key.
 *
 * Since the wizard → Preferences arch change, provider API-key entry
 * lives at /settings/providers (not the wizard's /setup/inference).
 * The daemon endpoints (/api/gateway/routers/{id}/key) are the same;
 * only the surface moved. Settings has no Continue mechanic, so the
 * key-write flow triggers on the daemon's save endpoint directly (the
 * ProvidersSection wires the same wizardState methods).
 *
 * Uses the daemon's actual /api/gateway/routers/{id}/key endpoints
 * for setup/teardown to avoid touching real OpenAI keys in the user's
 * Keychain — we only ever set/clear a known-fake key against the dev
 * daemon's namespace.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

async function clearRouterKey(id: string): Promise<void> {
  await fetch(`${DAEMON_URL}/api/gateway/routers/${id}/key`, { method: 'DELETE' });
}

async function setRouterKey(id: string, key: string): Promise<void> {
  await fetch(`${DAEMON_URL}/api/gateway/routers/${id}/key`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ key }),
  });
}

test.describe('Providers — Settings surface', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await clearRouterKey('openai');
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/settings/providers');
  });

  test('renders one card per known router with providers + capabilities chips', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await expect(card).toContainText('openai');
    await expect(card).toContainText('image_generate');
    await expect(card).toHaveAttribute('data-configured', 'false');
  });

  test('Clear button removes the key', async ({ tauriPage }) => {
    // Pre-set so the Clear button renders on hydrate.
    await setRouterKey('openai', 'sk-pretend');
    await navigateTo(tauriPage, '/logs');
    await new Promise(r => setTimeout(r, 800));
    await navigateTo(tauriPage, '/settings/providers');

    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    await expect(card).toHaveAttribute('data-configured', 'true', { timeout: 10_000 });

    await tauriPage.locator('[data-testid="router-clear-openai"]').click();
    await expect(card).toHaveAttribute('data-configured', 'false', { timeout: 5_000 });
  });
});
