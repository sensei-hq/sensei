import { test, expect, type Page } from '@playwright/test';

async function goToFirstLib(page: Page): Promise<boolean> {
  await page.goto('/libraries');
  await page.waitForLoadState('networkidle');
  const firstCard = page.locator('a[href^="/libraries/"]').first();
  if (await firstCard.count() === 0) return false;
  await firstCard.click();
  await page.waitForURL(/\/libraries\/[a-z0-9-]+$/);
  await page.waitForLoadState('networkidle');
  return true;
}

test.describe('Library detail page', () => {
  test('shows Edit, Re-index / Re-index CLI only, and Simulate buttons', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await expect(page.locator('button').filter({ hasText: /edit/i }).first()).toBeVisible();
    await expect(page.locator('button').filter({ hasText: /simulate/i }).first()).toBeVisible();
    // Either Re-index or Re-index (CLI only) visible
    const reindex = page.locator('button, span').filter({ hasText: /re-index/i }).first();
    await expect(reindex).toBeVisible();
  });

  test('Simulate button opens sidebar', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await page.locator('button').filter({ hasText: /simulate/i }).first().click();
    await expect(page.locator('textarea[name="query"]')).toBeVisible();
  });

  test('Edit button opens edit sidebar', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await page.locator('button').filter({ hasText: /^edit$/i }).first().click();
    await expect(page.locator('input[name="url"]')).toBeVisible();
    await expect(page.locator('select[name="category"]')).toBeVisible();
  });

  test('closing Simulate sidebar hides textarea', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    await page.locator('button').filter({ hasText: /simulate/i }).first().click();
    await expect(page.locator('textarea[name="query"]')).toBeVisible();

    // Close via X button
    await page.locator('button[aria-label="Close"]').first().click();
    await expect(page.locator('textarea[name="query"]')).not.toBeVisible();
  });

  test('stat cards display Sections, Repos, Queries, Last Indexed', async ({ page }) => {
    const found = await goToFirstLib(page);
    if (!found) { test.skip(true, 'No libraries in DB'); return; }

    const main = page.locator('main');
    await expect(main.getByText('Sections', { exact: true })).toBeVisible();
    // Use first() to avoid strict mode violation when 'Repos' appears in nav too
    await expect(main.getByText('Repos', { exact: true }).first()).toBeVisible();
    await expect(main.getByText('Queries', { exact: true })).toBeVisible();
    await expect(main.getByText('Last Indexed', { exact: true })).toBeVisible();
  });
});
