/**
 * Verification screenshots for the rebuilt observatory rail + new screens.
 * Captures to /tmp/sensei-shots/*.png for eyeballing against the mockups
 * (docs/mockups/Sensei/lib/observatory.jsx, impact.jsx, traceability.jsx,
 * upgrades.jsx). Not a regression gate. Run targeted:
 *   bun run test:e2e -- e2e/tests/zz-rail.spec.ts
 * Named zz- so it runs after the daemon is warm.
 */
import { test } from '../fixtures';
import { navigateTo } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const settle = (ms = 2000) => new Promise<void>((r) => setTimeout(r, ms));

test.describe('rail + new screens', () => {
  test('capture rail (all/focus) and the three new screens', async ({ tauriPage }) => {
    mkdirSync(DIR, { recursive: true });

    // Observatory root → the rail in All mode (every group visible).
    await navigateTo(tauriPage, '/');
    await settle();
    await tauriPage.screenshot({ path: `${DIR}/rail-all.png` });

    // Toggle Focus → rail collapses to anchors + "Needs you".
    await tauriPage.evaluate(`
      (function () {
        var btns = Array.from(
          document.querySelectorAll('[data-component="observatory-sidebar"] button'),
        );
        var focus = btns.find(function (b) { return (b.textContent || '').trim() === 'Focus'; });
        if (focus) focus.click();
      })()
    `);
    await settle(1200);
    await tauriPage.screenshot({ path: `${DIR}/rail-focus.png` });

    // The three new screens (stub pages — PageHeader + EmptyState).
    for (const route of ['impact', 'traceability', 'upgrades']) {
      await navigateTo(tauriPage, `/${route}`);
      await settle();
      await tauriPage.screenshot({ path: `${DIR}/${route}.png` });
    }
  });
});
