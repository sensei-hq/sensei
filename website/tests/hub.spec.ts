import { test, expect } from '@playwright/test';

test.describe('Sensei HQ hub at /', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('renders the hub title and key sections', async ({ page }) => {
    await expect(page).toHaveTitle(/Sensei HQ/);
    await expect(page.getByText('Four tools, one workshop')).toBeVisible();
    await expect(page.getByText('Still taking shape')).toBeVisible();
  });

  test('Sensei product card links internally to /sensei', async ({ page }) => {
    await expect(page.locator('a[href="/sensei"]').first()).toBeVisible();
  });

  test('external product cards point at per-product subdomains in a new tab', async ({ page }) => {
    for (const host of ['dbd', 'rokkit', 'kavach']) {
      const link = page.locator(`a[href="https://${host}.sensei-hq.com"]`).first();
      await expect(link).toBeVisible();
      await expect(link).toHaveAttribute('target', '_blank');
      await expect(link).toHaveAttribute('rel', /noopener/);
    }
  });
});
