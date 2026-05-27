/**
 * Inference stage — router list, paste a key, Continue → daemon
 * Keychain write. Clear button removes the key.
 *
 * Uses the daemon's actual /api/gateway/routers/{id}/key endpoints
 * for setup/teardown to avoid touching real OpenAI keys in the user's
 * actual Keychain — we only ever set/clear a known-fake key against
 * the dev daemon's namespace.
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

async function resetSetupKeys(): Promise<void> {
  const keys = [
    'setup.welcome', 'setup.preferences', 'setup.assistants',
    'setup.roots', 'setup.scan', 'setup_complete',
  ];
  for (const k of keys) {
    await fetch(`${DAEMON_URL}/api/config/${k}`, { method: 'DELETE' });
  }
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

test.describe('Inference stage', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await resetSetupKeys();
    await clearRouterKey('openai');
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/setup/inference');
  });

  test('renders one card per known router with providers + capabilities chips', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await expect(card).toContainText('openai');
    await expect(card).toContainText('image_generate');
    await expect(card).toHaveAttribute('data-configured', 'false');
  });

  test('paste key → Continue → daemon reports configured', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    const input = tauriPage.locator('[data-testid="router-key-input-openai"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await input.fill('sk-e2e-test-key');

    await tauriPage.locator('.btn-primary').click();

    // After commit, the daemon's view of the router should be configured.
    const deadline = Date.now() + 10_000;
    let configured = false;
    while (Date.now() < deadline) {
      const list = await fetch(`${DAEMON_URL}/api/gateway/routers`).then(r => r.json());
      const openai = list.routers.find((r: any) => r.id === 'openai');
      if (openai?.configured) { configured = true; break; }
      await new Promise(r => setTimeout(r, 200));
    }
    expect(configured).toBe(true);

    // Cleanup so subsequent test runs start fresh.
    await clearRouterKey('openai');
  });

  test('Clear button removes the key', async ({ tauriPage }) => {
    // Pre-set so the Clear button renders.
    await setRouterKey('openai', 'sk-pretend');
    await navigateTo(tauriPage, '/logs');
    await new Promise(r => setTimeout(r, 800));
    await navigateTo(tauriPage, '/setup/inference');

    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    await expect(card).toHaveAttribute('data-configured', 'true', { timeout: 10_000 });

    await tauriPage.locator('[data-testid="router-clear-openai"]').click();
    await expect(card).toHaveAttribute('data-configured', 'false', { timeout: 5_000 });
  });
});
