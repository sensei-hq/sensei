import { test, expect } from '@playwright/test';

test.describe('Dark mode switching', () => {
  test.beforeEach(async ({ page }) => {
    // Clear stored theme to get a clean state
    await page.addInitScript(() => {
      localStorage.removeItem('sensei-theme');
    });
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('page has data-mode and data-style on body', async ({ page }) => {
    const mode = await page.evaluate(() => document.body.dataset.mode);
    const style = await page.evaluate(() => document.body.dataset.style);
    expect(mode).toBeTruthy();
    expect(style).toBeTruthy();
  });

  test('dark mode applies dark surface colors to FloatingNavigation', async ({ page }) => {
    // Force light first to get a baseline
    await page.evaluate(() => { document.body.dataset.mode = 'light'; });

    const lightRing = await page.evaluate(() => {
      const el = document.querySelector('[data-floating-nav]');
      if (!el) return 'not-found';
      return getComputedStyle(el).getPropertyValue('--un-ring-color').trim();
    });

    // Switch to dark
    await page.evaluate(() => { document.body.dataset.mode = 'dark'; });

    const darkRing = await page.evaluate(() => {
      const el = document.querySelector('[data-floating-nav]');
      if (!el) return 'not-found';
      return getComputedStyle(el).getPropertyValue('--un-ring-color').trim();
    });

    // If FloatingNav exists, ring color must change between modes
    if (lightRing !== 'not-found') {
      expect(lightRing).not.toBe(darkRing);
    }
  });

  test('dark mode applies dark border to Toggle', async ({ page }) => {
    await page.evaluate(() => { document.body.dataset.mode = 'light'; });

    const lightBorder = await page.evaluate(() => {
      const el = document.querySelector('[data-toggle]');
      return el ? getComputedStyle(el).borderColor : 'not-found';
    });

    await page.evaluate(() => { document.body.dataset.mode = 'dark'; });

    const darkBorder = await page.evaluate(() => {
      const el = document.querySelector('[data-toggle]');
      return el ? getComputedStyle(el).borderColor : 'not-found';
    });

    if (lightBorder !== 'not-found') {
      expect(lightBorder).not.toBe(darkBorder);
    }
  });

  test.fixme('ThemeSwitcherToggle click switches to dark mode', async ({ page }) => {
    // Click the last toggle item (dark) via JS — items may be icon-only and tiny
    await page.evaluate(() => {
      const items = document.querySelectorAll('[data-toggle] [data-path]');
      if (items.length > 0) items[items.length - 1].click();
    });

    await page.waitForFunction(
      () => document.body.dataset.mode === 'dark',
      null,
      { timeout: 3000 }
    );

    const mode = await page.evaluate(() => document.body.dataset.mode);
    expect(mode).toBe('dark');
  });

  test.fixme('ThemeSwitcherToggle round-trip: dark then light', async ({ page }) => {
    // Click dark (last item)
    await page.evaluate(() => {
      const items = document.querySelectorAll('[data-toggle] [data-path]');
      if (items.length > 0) items[items.length - 1].click();
    });
    await page.waitForFunction(() => document.body.dataset.mode === 'dark', null, { timeout: 3000 });

    // Click light (middle item)
    await page.evaluate(() => {
      const items = document.querySelectorAll('[data-toggle] [data-path]');
      if (items.length >= 2) items[1].click();
    });
    await page.waitForFunction(() => document.body.dataset.mode === 'light', null, { timeout: 3000 });

    const mode = await page.evaluate(() => document.body.dataset.mode);
    expect(mode).toBe('light');
  });

  test('compound selector: dark styles apply when data-mode and data-style are on same element', async ({ page }) => {
    // This is the core regression test for rokkit#137
    const sameElement = await page.evaluate(() => {
      const body = document.body;
      return body.hasAttribute('data-mode') && body.hasAttribute('data-style');
    });
    expect(sameElement).toBe(true);

    // Force dark via attribute (bypasses toggle)
    await page.evaluate(() => { document.body.dataset.mode = 'dark'; });

    // Check that ANY themed element changes — use a broad check on the page background
    const darkBg = await page.evaluate(() => {
      // Check a themed element that's always visible
      const navBar = document.querySelector('nav');
      if (!navBar) return 'no-nav';
      const bg = getComputedStyle(navBar).backgroundColor;
      return bg;
    });

    // Switch back to light
    await page.evaluate(() => { document.body.dataset.mode = 'light'; });

    const lightBg = await page.evaluate(() => {
      const navBar = document.querySelector('nav');
      if (!navBar) return 'no-nav';
      return getComputedStyle(navBar).backgroundColor;
    });

    // Dark and light backgrounds must differ — if they're the same,
    // the dark mode CSS selectors aren't matching
    if (darkBg !== 'no-nav') {
      expect(darkBg).not.toBe(lightBg);
    }
  });
});
