import { test, expect } from '@playwright/test';

test.describe('Torii · Seiki product page at /torii-seiki', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/torii-seiki');
    await page.waitForLoadState('networkidle');
  });

  test('renders the title, hero and both client sections', async ({ page }) => {
    await expect(page).toHaveTitle(/Torii · Seiki/);
    await expect(
      page.getByRole('heading', { name: 'The gate, and the sanctuary behind it.' })
    ).toBeVisible();
    await expect(page.locator('#torii')).toBeVisible();
    await expect(page.locator('#seiki')).toBeVisible();
  });

  test('links back to the studio home and out to Gateway', async ({ page }) => {
    await expect(page.locator('a[href="/"]').first()).toBeVisible();
    const gateway = page.locator('a[href="https://gateway.sensei-hq.com"]').first();
    await expect(gateway).toBeVisible();
    await expect(gateway).toHaveAttribute('target', '_blank');
    await expect(gateway).toHaveAttribute('rel', /noopener/);
  });

  test('uses the sensei-HQ studio mark, not the sensei product mark', async ({ page }) => {
    // HQ studio pages carry the `sensei-hq` brushed-ensō; the product mark
    // (`sensei`) belongs only to the Sensei product surfaces.
    await expect(page.locator('[class~="i-brand:sensei-hq"]').first()).toBeVisible();
    await expect(page.locator('[class~="i-brand:sensei"]')).toHaveCount(0);
  });
});
