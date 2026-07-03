/**
 * Settings editable coverage — post-T3:
 *   • General tab: display name is now editable; auto-saves to
 *     daemon config (`user_name` + `setup.preferences.displayName`).
 *   • Inference tab: role → chain picker is now live; auto-saves via
 *     the Slice A endpoints (GET /api/gateway/chains + PUT
 *     /api/gateway/chains/{id}/role).
 *
 * Each test walks the DOM the user hits and verifies the daemon
 * reflects the change — same "poll the API" pattern the T3 flows spec
 * uses.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

async function safeJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(url);
    if (!res.ok) return fallback;
    const text = await res.text();
    if (!text) return fallback;
    return JSON.parse(text) as T;
  } catch {
    return fallback;
  }
}

test.describe('Settings — General editable', () => {
  test('typing a new display name persists to daemon config', async ({ tauriPage }) => {
    // Capture what the daemon holds now so we can restore in `finally`.
    const before = await safeJson<Record<string, string>>(`${DAEMON_URL}/api/config`, {});
    const original = before['user_name'] ?? '';

    await navigateTo(tauriPage, '/settings');
    await tauriPage.waitForSelector('[data-testid="pref-display-name"]', 15_000);

    const target = `e2e-user-${Date.now()}`;
    try {
      // The tauri-playwright wrapper's `fill` handles inputs; use it to
      // trigger the oninput handler that calls persist().
      await tauriPage.locator('[data-testid="pref-display-name"]').fill(target);

      // Poll the daemon config until the new name lands. The banner
      // switches to 'saved' asynchronously; we watch the wire truth.
      let seen = '';
      for (let i = 0; i < 20; i++) {
        const cfg = await safeJson<Record<string, string>>(`${DAEMON_URL}/api/config`, {});
        seen = cfg['user_name'] ?? '';
        if (seen === target) break;
        await new Promise<void>(r => setTimeout(r, 500));
      }
      expect(seen).toBe(target);
    } finally {
      // Cleanup — restore original name so re-runs stay deterministic.
      // Uses the daemon endpoint directly (not the UI) so an unexpected
      // page state during teardown doesn't leak the test value.
      try {
        await fetch(`${DAEMON_URL}/api/config`, {
          method: 'PUT', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ 'user_name': original }),
        });
      } catch { /* non-fatal */ }
    }
  });
});

test.describe('Settings — Inference tab', () => {
  test('role → chain picker renders each of the four supported roles', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/settings');
    // Click the Inference tab. `TabBar` uses text — walk the DOM to
    // find the tab labelled "Inference" and click it. Falls back to
    // waiting on the panel testid so a re-labelled tab isn't fatal.
    await tauriPage.evaluate(`
      (function() {
        var tab = Array.from(document.querySelectorAll('button, a'))
          .find(function(el) { return (el.textContent || '').trim() === 'Inference'; });
        if (tab) tab.click();
        return true;
      })()
    `);
    await tauriPage.waitForSelector('[data-testid="settings-inference"]', 15_000);

    for (const role of ['inference', 'consolidation', 'embedding', 'voice']) {
      const picker = tauriPage.locator(`[data-testid="inference-role-picker-${role}"]`);
      await expect(picker).toBeVisible({ timeout: 5_000 });
    }
  });

  test('clearing a role via the picker persists to the daemon', async ({ tauriPage }) => {
    // Ensure `voice` is unassigned at start of test so the two-phase clear
    // path is exercised on a chain we can safely mutate.
    const chains = await safeJson<{ chains: Array<{ id: string; name: string; role: string | null; capability: string }> }>(
      `${DAEMON_URL}/api/gateway/chains`, { chains: [] },
    );
    // Reasoning chain is seeded to `inference`. That's our test target —
    // we set it to null via the UI, verify, then restore.
    const reasoning = chains.chains.find(c => c.name === 'reasoning');
    if (!reasoning) { test.skip(true, 'reasoning chain missing on this daemon'); return; }
    if (reasoning.role !== 'inference') {
      test.skip(true, `expected reasoning chain to hold inference role, got ${reasoning.role}`);
      return;
    }

    await navigateTo(tauriPage, '/settings');
    await tauriPage.evaluate(`
      (function() {
        var tab = Array.from(document.querySelectorAll('button, a'))
          .find(function(el) { return (el.textContent || '').trim() === 'Inference'; });
        if (tab) tab.click();
        return true;
      })()
    `);
    await tauriPage.waitForSelector('[data-testid="inference-role-picker-inference"]', 15_000);

    // The wrapper's `selectOption` isn't reliable across all envs. Set
    // the value via evaluate() and dispatch a change event — that's what
    // the panel's onchange listens for.
    await tauriPage.evaluate(`
      (function() {
        var sel = document.querySelector('[data-testid="inference-role-picker-inference"]');
        if (sel) {
          sel.value = '';
          sel.dispatchEvent(new Event('change', { bubbles: true }));
        }
        return true;
      })()
    `);

    // Poll: reasoning chain's role should transition to null.
    let seenRole: string | null = 'inference';
    for (let i = 0; i < 20; i++) {
      const fresh = await safeJson<{ chains: Array<{ id: string; role: string | null; name: string }> }>(
        `${DAEMON_URL}/api/gateway/chains`, { chains: [] },
      );
      const now = fresh.chains.find(c => c.id === reasoning.id);
      seenRole = now?.role ?? null;
      if (seenRole == null) break;
      await new Promise<void>(r => setTimeout(r, 500));
    }
    expect(seenRole).toBeNull();

    // Restore reasoning → inference so the next test/run isn't skipped.
    try {
      const res = await fetch(`${DAEMON_URL}/api/gateway/chains/${reasoning.id}/role`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role: 'inference' }),
      });
      if (!res.ok) console.warn('[settings-editable] restore failed', res.status);
    } catch (e) { console.warn('[settings-editable] restore threw', e); }
  });
});
