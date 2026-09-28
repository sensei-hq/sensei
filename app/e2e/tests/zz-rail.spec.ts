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
  test('capture the rail and the three new screens', async ({ tauriPage }) => {
    mkdirSync(DIR, { recursive: true });

    // Observatory root → the whole rail. There is no longer an All|Focus
    // toggle to capture a second state for: it was dropped, so the rail has
    // one appearance.
    await navigateTo(tauriPage, '/');
    await settle();
    await tauriPage.screenshot({ path: `${DIR}/rail.png` });

    // The three new screens (stub pages — PageHeader + EmptyState).
    for (const route of ['impact', 'traceability', 'upgrades']) {
      await navigateTo(tauriPage, `/${route}`);
      await settle();
      await tauriPage.screenshot({ path: `${DIR}/${route}.png` });
    }
  });
});
