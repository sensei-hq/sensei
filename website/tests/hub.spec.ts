import { test, expect } from '@playwright/test';

test.describe('Sensei HQ hub at /', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('renders the hub title and key sections', async ({ page }) => {
    await expect(page).toHaveTitle(/Sensei HQ/);
    await expect(
      page.getByRole('heading', { name: 'Three products. Four libraries.' })
    ).toBeVisible();
    await expect(
      page.getByRole('heading', { name: 'What our products stand on.' })
    ).toBeVisible();
    await expect(page.getByText('Still taking shape')).toBeVisible();
  });

  test('product cards link internally (Sensei, Torii, Seiki)', async ({ page }) => {
    await expect(page.locator('a[href="/sensei"]').first()).toBeVisible();
    await expect(page.locator('a[href="/torii-seiki"]').first()).toBeVisible();
    await expect(page.locator('a[href="/torii-seiki#seiki"]').first()).toBeVisible();
  });

  test('library rows point at per-library subdomains in a new tab', async ({ page }) => {
    for (const host of ['gateway', 'dbd', 'rokkit', 'kavach']) {
      const link = page.locator(`a[href="https://${host}.sensei-hq.com"]`).first();
      await expect(link).toBeVisible();
      await expect(link).toHaveAttribute('target', '_blank');
      await expect(link).toHaveAttribute('rel', /noopener/);
    }
  });

  test('nav exposes both Products and Libraries', async ({ page }) => {
    await expect(page.locator('nav a[href="#products"]').first()).toBeVisible();
    await expect(page.locator('nav a[href="#libraries"]').first()).toBeVisible();
  });
});
