/**
 * Database gate E2E tests — real Sensei.app, real IPC.
 *
 * Tests the database gate on the /health bootstrap page.
 * Requires PostgreSQL running locally to reach 'ready'.
 * If PostgreSQL is absent the gate reaches 'blocked' — both are valid terminal states.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

// The database gate's row exposes its state as data-gate-status. `.status-pill`
// no longer exists: the first test passed vacuously against a missing element
// and the second timed out reading it.
const DB_GATE = '[data-component="gate-row"][data-gate-id="database"]';
const TRANSIENT = ['pending', 'checking', 'installing'];
// `?auto=false` keeps the screen up: on a healthy machine it would otherwise
// leave for / before the gate is read.

async function terminalStatus(tauriPage: { locator: (s: string) => { getAttribute: (n: string) => Promise<string | null> } }): Promise<string> {
  const deadline = Date.now() + 20_000;
  for (;;) {
    const status = await tauriPage.locator(DB_GATE).getAttribute('data-gate-status');
    if (status && !TRANSIENT.includes(status)) return status;
    if (Date.now() > deadline) throw new Error(`database gate still '${status}' after 20s`);
    await new Promise((r) => setTimeout(r, 250));
  }
}

test.describe('Bootstrap — database gate', () => {
  test('database gate reaches a terminal state', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health?auto=false');
    expect(['ready', 'failed']).toContain(await terminalStatus(tauriPage));
  });

  test('page does not crash when DB gate is blocked', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health?auto=false');
    if ((await terminalStatus(tauriPage)) === 'failed') {
      // Remedy UI must be visible when the gate is blocked.
      await expect(tauriPage.locator('[data-component="remedy"]')).toBeVisible();
    }
    // If 'ready', no remedy shown — that is fine.
  });
});
