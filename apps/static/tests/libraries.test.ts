import { test, expect } from '@playwright/test';

test.describe('Libraries page', () => {
  test('loads and shows grid view by default', async ({ page }) => {
    await page.goto('/libraries');
    await page.waitForLoadState('networkidle');
    // Grid view toggle button visible
    await expect(page.locator('button[title="Table view"]')).toBeVisible({ timeout: 10_000 });
  });

  test('Add Library button opens sidebar', async ({ page }) => {
    await page.goto('/libraries');
    await page.waitForLoadState('networkidle');
    const addBtn = page.locator('button').filter({ hasText: /add library/i }).first();
    await expect(addBtn).toBeVisible({ timeout: 10_000 });
    await addBtn.click();
    // Sidebar form appears
    await expect(page.locator('input[name="name"]')).toBeVisible();
    await expect(page.locator('input[name="url"]')).toBeVisible();
  });

  test('Add Library sidebar closes on Cancel', async ({ page }) => {
    await page.goto('/libraries');
    await page.waitForLoadState('networkidle');
    await page.locator('button').filter({ hasText: /add library/i }).first().click();
    await expect(page.locator('input[name="name"]')).toBeVisible();
    await page.locator('button').filter({ hasText: /cancel/i }).click();
    await expect(page.locator('input[name="name"]')).not.toBeVisible();
  });

  test('Add Library sidebar closes on overlay click', async ({ page }) => {
    await page.goto('/libraries');
    await page.waitForLoadState('networkidle');
    await page.locator('button').filter({ hasText: /add library/i }).first().click();
    await expect(page.locator('input[name="name"]')).toBeVisible();
    // Click overlay element directly
    await page.locator('[role="presentation"]').click();
    await expect(page.locator('input[name="name"]')).not.toBeVisible();
  });

  test('library cards link to detail page', async ({ page }) => {
    await page.goto('/libraries');
    await page.waitForLoadState('networkidle');
    const firstCard = page.locator('a[href^="/libraries/"]').first();
    const count = await firstCard.count();
    if (count === 0) {
      test.skip(true, 'No libraries in DB — skipping navigation test');
      return;
    }
    const href = await firstCard.getAttribute('href');
    await firstCard.click();
    await expect(page).toHaveURL(href!);
  });
});
