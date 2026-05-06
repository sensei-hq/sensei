/**
 * Database gate E2E tests — real Sensei.app, real IPC.
 *
 * Tests the database gate on the /health bootstrap page.
 * Requires PostgreSQL running locally to reach 'ready'.
 * If PostgreSQL is absent the gate reaches 'blocked' — both are valid terminal states.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

test.describe('Bootstrap — database gate', () => {
  test('database gate reaches a terminal state', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');

    const dbPill = tauriPage
      .locator('.gate-row')
      .filter({ hasText: /database/i })
      .locator('.status-pill');

    // Wait up to 20 s for the pill to leave transient states
    await expect(dbPill).not.toContainText(/waiting|checking/, { timeout: 20_000 });
  });

  test('page does not crash when DB gate is blocked', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');

    const dbPill = tauriPage
      .locator('.gate-row')
      .filter({ hasText: /database/i })
      .locator('.status-pill');

    // Determine final state (allow up to 20 s)
    await expect(dbPill).not.toContainText(/waiting|checking/, { timeout: 20_000 });

    const pillText = await dbPill.innerText();
    if (pillText.includes('blocked')) {
      // Remedy UI must be visible when gate is blocked
      await expect(tauriPage.locator('.remedy')).toBeVisible();
    }
    // If 'ready', no remedy shown — that is fine
  });
});
