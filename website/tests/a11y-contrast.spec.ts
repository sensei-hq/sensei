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
      await page.goto(path);
      await page.waitForLoadState('networkidle');
      await page.evaluate((m) => {
        document.documentElement.dataset.mode = m;
        document.body.dataset.mode = m;
      }, mode);

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
