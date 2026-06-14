import { test, expect } from '@playwright/test';

test('Sensei product site renders at /sensei', async ({ page }) => {
  await page.goto('/sensei');
  await expect(page).toHaveTitle(/Sensei —/);
});

test.describe('Sensei sub-pages + org legal pages render', () => {
  for (const path of ['/sensei/docs', '/sensei/faq', '/privacy', '/terms']) {
    test(`renders ${path}`, async ({ page }) => {
      const resp = await page.goto(path);
      expect(resp?.status() ?? 200).toBeLessThan(400);
    });
  }
});
