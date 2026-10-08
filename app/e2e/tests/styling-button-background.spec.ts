/**
 * A background utility must be able to paint a `<button type="button">` (#244).
 *
 * This is a CASCADE invariant, so it can only be checked in a browser: every
 * class is present in the markup and the computed value is what differs. A unit
 * test on the component would assert the class list and pass while the pixel
 * stayed transparent, which is exactly how this shipped.
 *
 * ## What it was
 *
 * `@unocss/reset/tailwind.css` sets
 * `button, [type="button"], [type="reset"], [type="submit"] { background-color:
 * transparent }`. `[type="button"]` is an ATTRIBUTE selector — specificity
 * (0,1,0), the same as `.bg-ink` or `.bg-primary` — and the reset came later, so
 * the tie went to the reset and the utility lost. Measured on project Sessions:
 * the selected "7 days" chip carried `bg-ink text-paper` and computed
 * `rgba(0, 0, 0, 0)` behind `oklch(0.975 0.008 85)`, near-white on paper. 13
 * such sites across seven screens.
 *
 * A `<button>` with NO type attribute was never affected — the bare `button`
 * selector is (0,0,1) and loses on specificity — which is why the identical chip
 * on Observatory Instruments rendered correctly, and why this looked like a
 * per-screen styling problem rather than one rule.
 *
 * The fix is one line in `src/app.css`: the reset is imported into a cascade
 * layer, and unlayered declarations beat layered ones whatever their
 * specificity.
 *
 * ## Why the assertion is generic
 *
 * It walks whatever the page actually rendered rather than naming one chip. A
 * test pinned to "the 7 days button" would go green the day that screen is
 * rewritten, while the invariant it stands for quietly broke everywhere else.
 */

import { test, expect } from '../fixtures';
import { navigateTo, navigateToScreen, installErrorTrap, DAEMON_URL, seedSetupComplete } from '../helpers';


/** Buttons whose class asks for a background, and what they actually painted. */
const AUDIT = `
  (function () {
    var out = { checked: 0, transparent: [] };
    var wanted = /\\bbg-(ink|paper|paper-soft|paper-mute|primary|accent|accent-soft|success|warning|danger|info)\\b/;
    document.querySelectorAll('button[type="button"]').forEach(function (b) {
      var cls = String(b.className || '');
      if (!wanted.test(cls)) return;
      out.checked += 1;
      var bg = getComputedStyle(b).backgroundColor;
      // Fully transparent means the utility lost. A translucent token value is
      // a real colour and is left alone.
      if (bg === 'rgba(0, 0, 0, 0)' || bg === 'transparent') {
        out.transparent.push((b.textContent || '').trim().slice(0, 24) + ' :: ' + cls.slice(-70));
      }
    });
    return JSON.stringify(out);
  })()
`;

test.describe('Styling · a background utility paints a typed button', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(120_000);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await installErrorTrap(tauriPage);
  });

  test('no typed button asks for a background and renders transparent', async ({ tauriPage }) => {
    // Observatory Sessions renders its range chips from static copy, so they are
    // present whether or not the throwaway database holds a session.
    await navigateToScreen(tauriPage, '/sessions', '[data-screen="sessions"], main');

    const audit = JSON.parse(String(await tauriPage.evaluate(AUDIT))) as {
      checked: number;
      transparent: string[];
    };

    // SKIP RATHER THAN PASS when the page rendered nothing to check — a green
    // tick on zero elements is the vacuous assertion this whole file exists to
    // avoid.
    test.skip(audit.checked === 0, 'this screen rendered no typed button asking for a background');

    expect(
      audit.transparent,
      `a background utility lost to the preflight on ${audit.transparent.length} of ` +
        `${audit.checked} buttons — see #244, the reset must stay in a cascade layer`,
    ).toEqual([]);
  });
});
