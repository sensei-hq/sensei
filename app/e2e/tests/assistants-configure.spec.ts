/**
 * Per-family assistant configuration flow (post-mockup rehab).
 *
 * The card now fires configure / remove the moment the switch toggles —
 * matching docs/mockups/lib/assistant-tick-options.jsx. Continue persists
 * setup.assistants=done and navigates; commitStage is a no-op for any
 * family that's already converged because of the eager toggle.
 *
 * Verifies:
 *   1. Card renders with per-part chips + a switch with .on class when enabled.
 *   2. Toggling the switch fires configure → SSE drives chips through
 *      configuring → done → data-configured flips to true.
 *   3. Toggling off a configured family fires remove → chips return to idle.
 *   4. Continue persists setup.assistants=done and routes to /setup/roots.
 *
 * The daemon is real (e2e daemon on port 7744 against the sensei_e2e DB,
 * driven by SENSEI_INSTANCE=e2e in globalSetup). The user's prod config is
 * not touched.
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

  test('card renders with switch and per-part chips', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await expect(card.locator('button[aria-label="Enable Claude"]')).toBeVisible();
    // Claude has five capability parts: plugins, skills, commands, agents, mcp.
    // Lock the count so a future trim/expansion of the daemon's parts list
    // surfaces here instead of going unnoticed.
    await expect(card.locator('.chip')).toHaveCount(5, { timeout: 5_000 });
    // Slice starts every part idle until the daemon emits a transition.
    await expect(card).toHaveAttribute('data-configure-state', 'idle');
  });

  test('toggling the switch off does not fire configure (already-idle family stays idle)', async ({ tauriPage }) => {
    // Claude was put into not-configured state in beforeEach; the wizard
    // defaults `selected` to any-variant-installed = true. Toggling off
    // an unconfigured family is a no-op — there's nothing to remove.
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    const sw = card.locator('button[aria-label="Enable Claude"]');

    await expect(sw).toHaveClass(/\bon\b/, { timeout: 10_000 });
    await sw.click();
    await expect(sw).not.toHaveClass(/\bon\b/);
    await expect(card).toHaveAttribute('data-configure-state', 'idle');
  });

  test('toggling the switch on fires configure → chips settle to done', async ({ tauriPage }) => {
    // Start from a known-off state so the toggle has work to do.
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    const sw = card.locator('button[aria-label="Enable Claude"]');

    await expect(sw).toHaveClass(/\bon\b/, { timeout: 10_000 });
    await sw.click();  // off
    await expect(sw).not.toHaveClass(/\bon\b/);
    await sw.click();  // back on → triggers configure

    // The chip strip must visibly enter 'configuring' so the user sees
    // progress. SSE then settles each part to 'done' as the daemon
    // finishes the plugin install.
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configure-state', 'configuring', 5_000);
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configured', 'true', 30_000);
  });

  test('Continue persists setup.assistants=done and navigates to roots', async ({ tauriPage }) => {
    // After eager-toggle configures already landed, Continue is a thin
    // marker write + navigate.
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await tauriPage.locator('[data-action="next"]').click();
    await waitForPath(tauriPage, '/setup/roots', 20_000);

    const config = await fetch(`${DAEMON_URL}/api/config`).then(r => r.json()) as Record<string, string>;
    expect(config['setup.assistants']).toBe('done');
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

  test('toggling off a configured family fires remove → chips return to idle', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="assistant-card-claude"]');
    const sw = card.locator('button[aria-label="Enable Claude"]');

    await expect(card).toBeVisible({ timeout: 10_000 });
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configured', 'true');
    await expect(sw).toHaveClass(/\bon\b/);
    await sw.click();

    // Removal goes through the same 'configuring' visual state as install
    // (chips spin while the daemon's remove() runs), then settles to idle.
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configure-state', 'configuring', 5_000);
    await waitForAttr(tauriPage, '[data-testid="assistant-card-claude"]', 'data-configure-state', 'idle', 30_000);

    // Daemon's canonical state — at least one installed Claude variant
    // should report configured=false. (claude-desktop's MCP entry is
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
