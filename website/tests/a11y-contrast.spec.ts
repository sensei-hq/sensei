import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// WCAG 2.1 AA colour-contrast on every top-level page, in BOTH themes. The
// named-token palette must stay legible when it flips, so we run axe's
// `color-contrast` rule against each page with data-mode forced light and dark.
const PAGES = ['/', '/torii-seiki', '/sensei', '/sensei/docs', '/sensei/faq', '/privacy', '/terms'];
const MODES = ['light', 'dark'] as const;

for (const path of PAGES) {
  for (const mode of MODES) {
    test(`no WCAG AA colour-contrast violations on ${path} (${mode})`, async ({ page }) => {
      // Set the mode the way a real visitor does — a persisted theme applied at
      // load — so the theme CSS is fully resolved before axe measures. (Toggling
      // data-mode via runtime JS leaves component-layer colours on stale values.)
      await page.addInitScript((m) => {
        localStorage.setItem(
          'sensei-site-theme',
          JSON.stringify({ mode: m, style: 'zen-sumi', density: 'comfortable', skin: 'default', direction: 'ltr' })
        );
      }, mode);
      await page.goto(path);
      await page.waitForLoadState('networkidle');

      const results = await new AxeBuilder({ page }).withRules(['color-contrast']).analyze();

      // Surface each offending node (selector + measured ratio) so a failure is
      // actionable, not just a count.
      const offenders = results.violations.flatMap((v) =>
        v.nodes.map((n) => `${n.target.join(' ')} — ${n.failureSummary?.split('\n').pop()?.trim() ?? ''}`)
      );
      expect(offenders, `contrast failures:\n${offenders.join('\n')}`).toEqual([]);
    });
  }
}
