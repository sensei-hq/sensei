/**
 * Verification screenshots (not a regression gate) — captures key screens to
 * /tmp/sensei-shots/*.png so the implementation can be eyeballed against the
 * mockups. Run targeted: `bun run test:e2e -- e2e/tests/zz-screenshots.spec.ts`.
 * Named zz- so it runs last (after the daemon is warm).
 */
import { test } from '../fixtures';
import { navigateTo } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const settle = (ms = 1500) => new Promise<void>((r) => setTimeout(r, ms));

test.describe('verification screenshots', () => {
  test('capture health + observatory', async ({ tauriPage }) => {
    mkdirSync(DIR, { recursive: true });

    // Health — auto=false keeps the page from redirecting once the gate is green,
    // so we can see the wordmark + ledger.
    await navigateTo(tauriPage, '/health?auto=false');
    await settle(2500);
    await tauriPage.screenshot({ path: `${DIR}/health.png` });

    // Observatory (root) — the sidebar rail with the logo wordmark.
    await navigateTo(tauriPage, '/');
    await settle(2500);
    await tauriPage.screenshot({ path: `${DIR}/observatory.png` });
  });
});
