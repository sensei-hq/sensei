import { test, expect, type Locator } from '@playwright/test';

// Scrolling is the shell's job and the one that keeps breaking (CONTRACT §1: the title bar,
// the bands and the footer stay put; only the body scrolls). Every RelayShell fixture is
// drawn in a frame the height of the app's <main>, its body long enough to overflow, and
// this scrolls each one to its last row at both widths. A shell that grows past its frame,
// a frame that scrolls instead of the body, or a body that cannot reach its end fails here.

const WIDTHS = [
  { width: 'wide', viewport: { width: 1440, height: 2600 } },
  { width: 'phone', viewport: { width: 430, height: 1800 } },
] as const;

const top = async (l: Locator) => (await l.boundingBox())!.y;

for (const w of WIDTHS) {
  test(`every RelayShell fixture scrolls its body to the end, and nothing else moves (${w.width})`, async ({ page }) => {
    await page.setViewportSize(w.viewport);
    await page.goto(`/preview/RelayShell?width=${w.width}`);
    await page.locator('[data-preview-ready]').waitFor({ timeout: 60_000 });
    const sections = page.locator('[data-preview^="RelayShell/"]');
    const n = await sections.count();
    expect(n, 'the RelayShell fixtures are listed').toBeGreaterThan(0);

    for (let i = 0; i < n; i += 1) {
      const section = sections.nth(i);
      const id = await section.getAttribute('data-preview');
      await section.scrollIntoViewIfNeeded();
      const frame = section.locator('[data-preview-sample]');
      const shell = frame.locator('[data-screen-shell]');
      const body = shell.locator('[data-screen-body]');
      const header = shell.locator('[data-screen-header]');

      // The shell fills its frame, no taller: it is the frame that is the screen's height.
      const [frameBox, shellBox] = [(await frame.boundingBox())!, (await shell.boundingBox())!];
      expect(Math.round(shellBox.height), `${id}: shell height = frame height`).toBeLessThanOrEqual(Math.round(frameBox.height));

      // The body overflows (the fixture has rows enough) and is the scroller.
      const { scrollHeight, clientHeight } = await body.evaluate((el) => ({ scrollHeight: el.scrollHeight, clientHeight: el.clientHeight }));
      expect(scrollHeight, `${id}: the body overflows`).toBeGreaterThan(clientHeight);

      const headerTop = await top(header);
      await body.evaluate((el) => el.scrollTo({ top: el.scrollHeight }));
      await expect.poll(() => body.evaluate((el) => Math.ceil(el.scrollTop + el.clientHeight) >= el.scrollHeight), { message: `${id}: the body reaches its end` }).toBe(true);

      // The last row is inside the body's box; the header has not moved; the frame did not scroll.
      const last = body.locator('[data-scroll-row]').last();
      const [lastBox, bodyBox] = [(await last.boundingBox())!, (await body.boundingBox())!];
      expect(lastBox.y + lastBox.height, `${id}: the last row is in view`).toBeLessThanOrEqual(bodyBox.y + bodyBox.height + 1);
      expect(await top(header), `${id}: the header stays put`).toBeCloseTo(headerTop, 0);
      expect(await frame.evaluate((el) => el.scrollTop), `${id}: the frame does not scroll`).toBe(0);
      await expect(shell.locator('[data-screen-footer]').first(), `${id}: the footer is in view`).toBeInViewport();
    }
  });
}
