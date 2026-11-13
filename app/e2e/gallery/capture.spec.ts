import { test, expect } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Screenshots every `[data-gallery]` section of the dev-only /gallery page
// across the verification matrix (COMPONENTS.md §0: light/dark × wide/phone ×
// kanji/icon), one PNG per component fixture per variant, plus a manifest the
// contact sheet reads. The gallery renders fixtures, so a capture is a picture
// of the component alone — the thing the mockup crop is compared with.
//
// Output: GALLERY_OUT, else e2e/gallery/out (gitignored — no checked-in PNGs;
// evidence for a report is copied to coord/reports/<lane>/ from there).

const OUT = resolve(process.env.GALLERY_OUT ?? fileURLToPath(new URL('./out/', import.meta.url)), 'components');
const WIDTHS = ['wide', 'phone'] as const;
const THEMES = ['light', 'dark'] as const;
const GLYPHS = ['kanji', 'icon'] as const;

type Variant = { width: (typeof WIDTHS)[number]; theme: (typeof THEMES)[number]; glyph: (typeof GLYPHS)[number] };
const variants: Variant[] = WIDTHS.flatMap((width) =>
  THEMES.flatMap((theme) => GLYPHS.map((glyph) => ({ width, theme, glyph }))),
);
const slug = (id: string) => id.replace(/[^A-Za-z0-9]+/g, '-').replace(/^-|-$/g, '');

test('every gallery fixture is captured across the matrix', async ({ page }) => {
  mkdirSync(OUT, { recursive: true });
  const manifest: Record<string, { note: string; shots: Record<string, string> }> = {};
  for (const v of variants) {
    const name = `${v.width}-${v.theme}-${v.glyph}`;
    await page.setViewportSize({ width: v.width === 'phone' ? 430 : 1440, height: 1000 });
    await page.goto(`/gallery?theme=${v.theme}&glyph=${v.glyph}&width=${v.width}`);
    await page.locator('[data-gallery-ready]').waitFor({ timeout: 60_000 });
    await page.evaluate(() => document.fonts?.ready);
    const sections = page.locator('[data-gallery]');
    const n = await sections.count();
    expect(n, 'the gallery lists fixtures').toBeGreaterThan(0);
    for (let i = 0; i < n; i += 1) {
      const section = sections.nth(i);
      const id = (await section.getAttribute('data-gallery')) ?? `fixture-${i}`;
      const note = (await section.locator('[data-gallery-note]').textContent())?.trim() ?? '';
      const dir = resolve(OUT, slug(id));
      mkdirSync(dir, { recursive: true });
      await section.scrollIntoViewIfNeeded();
      const file = resolve(dir, `${name}.png`);
      await section.locator('[data-gallery-sample]').screenshot({ path: file, animations: 'disabled' });
      (manifest[id] ??= { note, shots: {} }).shots[name] = file;
    }
  }
  writeFileSync(resolve(OUT, 'manifest.json'), JSON.stringify(manifest, null, 2));
});
