/**
 * Reproduces the user-reported bug:
 *   "last Continue (on /setup/done) should take to observatory, but it
 *    lands me back on /setup/welcome."
 *
 * Test approach: navigate directly to /setup/done, click the Continue
 * button ("Enter observatory →"), and assert the URL becomes / (not
 * /setup/welcome and not still /setup/done).
 *
 * No pre-population of daemon config. The done-stage's commit handler is
 * what should write `setup_complete=1` and flip `wizardState.setupComplete`
 * — we're testing whether that sequence lands the user at /.
 */

import { test, expect } from '../fixtures';
import { DAEMON_URL } from '../helpers';

test.describe('Wizard done → observatory', () => {
  test('clicking Continue on /setup/done navigates to / (not /setup/welcome)', async ({ tauriPage }) => {
    // Clear any prior daemon state so we start from a clean slate.
    // (sensei_e2e is fresh per globalSetup; the daemon may have written
    // some setup.X markers in earlier in-session tests though.)
    await fetch(`${DAEMON_URL}/api/config/setup_complete`, { method: 'DELETE' });

    // Navigate directly to /setup/done via anchor click — SvelteKit
    // intercepts and routes via reroute. Anchor click is preferred over
    // location.href = '/setup/done' because the latter does a full
    // navigation that may not flow through reroute the same way.
    await tauriPage.evaluate(`(() => {
      const a = document.createElement('a');
      a.href = '/setup/done';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
    })()`);
    await new Promise(r => setTimeout(r, 1500));

    const beforeClick = await tauriPage.evaluate(`window.location.pathname`) as string;
    expect(beforeClick, 'expected to be on /setup/done before the click').toBe('/setup/done');

    // Click the Continue button — `[data-action="next"]` in (config)/+layout.svelte.
    // On the done stage it reads "Enter observatory →".
    await tauriPage.click('[data-action="next"]');

    // Wait for the daemon write + invalidateAll + goto + reroute to settle.
    await new Promise(r => setTimeout(r, 4000));

    const afterClick = await tauriPage.evaluate(`window.location.pathname`) as string;
    expect(afterClick, `final Continue should land at /, not ${afterClick}`)
      .toBe('/');

    // Sanity: the daemon should record setup_complete=1 after the click.
    const cfg = await fetch(`${DAEMON_URL}/api/config`).then(r => r.json());
    expect(cfg.setup_complete, 'daemon must have setup_complete=1 after done-stage Continue').toBe('1');
  });
});
