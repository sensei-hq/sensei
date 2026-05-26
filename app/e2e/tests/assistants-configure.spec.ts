/**
 * Per-family assistant configuration flow.
 *
 * Verifies that the Configure & Continue button:
 *   1. Iterates over selected families one at a time
 *   2. Flips each card's data-configure-state through configuring → done
 *   3. Persists setup.assistants=done in the daemon config
 *   4. Navigates to /setup/roots only after all configures succeed
 *
 * The daemon is real (dev daemon on port 7745). Claude is the only family
 * detected on the test machine; configure marks configured=true on both
 * claude-code and claude-desktop variants.
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

/**
 * Wait for the wizard hydrate cycle to populate variant state. The card itself
 * appears as soon as `wizardState.assistants.assistants.length > 0`, but the
 * data-configured attribute reflects post-hydrate variant.configured values
 * which can land a tick or two later in the live Tauri webview. Poll the DOM
 * directly — `toHaveAttribute` doesn't always pick this up reliably in the
 * tauri-playwright bridge.
 */
async function waitForAttr(
  tauriPage: any,
  selector: string,
  attr: string,
  expected: string,
  timeoutMs = 15_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const v = await tauriPage.evaluate(
      `document.querySelector(${JSON.stringify(selector)})?.getAttribute(${JSON.stringify(attr)})`,
    ).catch(() => null);
    if (v === expected) return;
    await new Promise(r => setTimeout(r, 150));
  }
  const final = await tauriPage.evaluate(
    `document.querySelector(${JSON.stringify(selector)})?.getAttribute(${JSON.stringify(attr)})`,
  ).catch(() => '(unknown)');
  throw new Error(`Timed out (${timeoutMs}ms) waiting for ${selector}[${attr}=${expected}]. Current: ${final}`);
}

async function waitForPath(tauriPage: any, expected: string, timeoutMs = 20_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const p = await tauriPage.evaluate(`window.location.pathname`).catch(() => null);
    if (p === expected) return;
    await new Promise(r => setTimeout(r, 100));
  }
  const final = await tauriPage.evaluate(`window.location.pathname`).catch(() => '(unknown)');
  throw new Error(`Timed out (${timeoutMs}ms) waiting for path ${expected}. Current: ${final}`);
}

async function resetSetupKeys(): Promise<void> {
  // Strip setup completion so the wizard starts fresh.
  const keys = [
    'setup.welcome', 'setup.preferences', 'setup.assistants',
    'setup.roots', 'setup.scan', 'setup_complete',
  ];
  for (const k of keys) {
    await fetch(`${DAEMON_URL}/api/config/${k}`, { method: 'DELETE' });
  }
}

async function unconfigureClaude(): Promise<void> {
  await fetch(`${DAEMON_URL}/api/assistants/remove`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ acps: ['claude-code', 'claude-desktop'] }),
  });
}

async function configureClaude(): Promise<void> {
  await fetch(`${DAEMON_URL}/api/assistants/configure`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ acps: ['claude-code', 'claude-desktop'] }),
  });
}

test.describe('Assistants — per-family configure', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await resetSetupKeys();
    // Put Claude in a known "not configured" state so each test exercises
    // the configure path (the wizard's reconcile logic correctly no-ops
    // when daemon state already matches user intent).
    await unconfigureClaude();
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/logs'); // force (config) layout remount on next nav
    await navigateTo(tauriPage, '/setup/assistants');
  });

  test('card renders with switch and capability chips', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await expect(card.locator('button[aria-label="Enable Claude"]')).toBeVisible();
    await expect(card.locator('.chip').first()).toBeVisible();
    await expect(card).toHaveAttribute('data-configure-state', 'idle');
  });

  test('switch toggles selected state without firing configure', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    const sw = card.locator('button[aria-label="Enable Claude"]');

    // Card starts selected (variants are installed → defaults to on)
    await expect(sw).toHaveClass(/\bon\b/);
    await sw.click();
    await expect(sw).not.toHaveClass(/\bon\b/);
    // State stays idle — toggling alone never calls configure
    await expect(card).toHaveAttribute('data-configure-state', 'idle');
  });

  test('Configure & Continue → card flips to configuring → navigates → daemon persists', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    await expect(card).toHaveAttribute('data-configure-state', 'idle');

    const button = tauriPage.locator('.btn-primary');
    await expect(button).toContainText('Configure');

    await button.click();

    // The card must visibly enter the 'configuring' state so the user sees
    // progress. (The 'done' state is transient — the card unmounts the same
    // tick as navigation fires, so we assert via URL + daemon state instead.)
    await expect(card).toHaveAttribute('data-configure-state', 'configuring', { timeout: 5_000 });

    // Navigation completes only when every selected family finished cleanly.
    await waitForPath(tauriPage, '/setup/roots', 20_000);

    // Daemon persisted the completion (setConfig setup.assistants=done is the
    // final step of commitStage and only runs if all configures succeeded).
    const config = await fetch(`${DAEMON_URL}/api/config`).then(r => r.json()) as Record<string, string>;
    expect(config['setup.assistants']).toBe('done');
  });

  test('after configure, every installed Claude variant reports configured=true', async ({ tauriPage }) => {
    // Wait for the card to render — confirms hydrate ran before we click Continue.
    await expect(tauriPage.locator('[data-testid="assistant-card-claude"]')).toBeVisible({ timeout: 10_000 });
    await tauriPage.locator('.btn-primary').click();
    await waitForPath(tauriPage, '/setup/roots', 20_000);

    const families = await fetch(`${DAEMON_URL}/api/assistants/families`).then(r => r.json()) as Array<{
      family: string;
      members: Array<{ id: string; installed: boolean; configured: boolean }>;
    }>;
    const claude = families.find(f => f.family === 'claude')!;
    const installed = claude.members.filter(m => m.installed);
    expect(installed.length).toBeGreaterThan(0);
    for (const variant of installed) {
      expect(variant.configured, `${variant.id} should be configured after the flow`).toBe(true);
    }
  });
});

test.describe('Assistants — re-entry and removal', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await resetSetupKeys();
    // Put Claude in a known "configured" state — these tests exercise what
    // happens when the user returns and sees the family already configured.
    await configureClaude();
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await navigateTo(tauriPage, '/setup/assistants');
  });

  test('re-entry shows "configured ✓" when daemon says the family is configured', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configured', 'true');
    await expect(card.locator('.mono').filter({ hasText: /configured/i })).toBeVisible({ timeout: 5_000 });
  });

  test('unchecking a configured family + Continue triggers removal', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    const sw = card.locator('button[aria-label="Enable Claude"]');

    await expect(card).toBeVisible({ timeout: 10_000 });
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configured', 'true');
    await expect(sw).toHaveClass(/\bon\b/);
    await sw.click();
    await expect(sw).not.toHaveClass(/\bon\b/);

    await tauriPage.locator('.btn-primary').click();

    // Card must visibly transition through removing (not configuring).
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configure-state', 'removing', 5_000);
    await waitForPath(tauriPage, '/setup/roots', 20_000);

    // Daemon's now-canonical state — at least one installed Claude variant
    // should report configured=false now. (claude-desktop's MCP entry is
    // cleanly removed by file edit; claude-code's plugin uninstall depends
    // on `claude` CLI being on PATH for the spawned daemon process.)
    const after = await fetch(`${DAEMON_URL}/api/assistants/families`).then(r => r.json()) as Array<{
      family: string;
      members: Array<{ id: string; installed: boolean; configured: boolean }>;
    }>;
    const claudeAfter = after.find(f => f.family === 'claude')!;
    const stillConfigured = claudeAfter.members.filter(m => m.installed && m.configured).length;
    const totalInstalled = claudeAfter.members.filter(m => m.installed).length;
    expect(stillConfigured).toBeLessThan(totalInstalled);
  });
});
