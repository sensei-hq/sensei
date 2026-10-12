/**
 * Settings · Assistants · Conversation history — consent to read transcripts (#218).
 *
 * The controller spec covers the switch logic against an injected API. This
 * covers what it cannot: the card mounting on the real screen with every source
 * the daemon knows, and a click reaching the daemon — the switch shows what the
 * daemon STORED, so the assertion is made against the daemon, not the DOM alone.
 *
 * Runs against the throwaway e2e daemon; the consent is reset to off first so a
 * previous run's leftovers cannot make this pass.
 */

import { test, expect } from '../fixtures';
import { navigateTo, daemonGet, DAEMON_URL, seedSetupComplete } from '../helpers';

type Row = { source: string; label: string; consented: boolean };

async function setConsent(source: string, consented: boolean): Promise<void> {
  await fetch(`${DAEMON_URL}/api/transcripts/consent/${source}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ consented }),
  });
}

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

test.describe('Settings · Assistants · Conversation history', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(120_000);
    await seedHealth(tauriPage);
    // AFTER seedHealth, which clears the local flag. Setup must be complete or
    // `hooks.ts` reroutes every page to setup, and a reset e2e DB has no
    // `setup_complete` row.
    await seedSetupComplete(tauriPage);
  });

  test('every source is listed, off, and a switch reaches the daemon', async ({ tauriPage }) => {
    const sources = await daemonGet<Row[]>('/api/transcripts/consent');
    for (const s of sources) await setConsent(s.source, false);

    await navigateTo(tauriPage, '/settings/assistants');
    await tauriPage.waitForSelector('[data-testid="transcript-consent-row"]', 30_000);
    expect(await tauriPage.count('[data-testid="transcript-consent-row"]')).toBe(sources.length);
    expect(await tauriPage.count('[data-testid="transcript-consent-error"]')).toBe(0);

    await tauriPage.locator('[data-source="zed"] [role="switch"]').click();
    await expect
      .poll(async () => (await daemonGet<Row[]>('/api/transcripts/consent')).find((r) => r.source === 'zed')?.consented, {
        timeout: 15_000,
      })
      .toBe(true);

    await setConsent('zed', false);
  });
});
