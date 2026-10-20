/**
 * #133 against the real app: choosing "Icons" in Settings → General switches the
 * settings rail to Solar icons that actually paint (non-zero size, an SVG mask
 * from the built CSS), the daemon stores `ui.glyphs = icon`, and choosing
 * "Kanji" switches back. No reload: the rails follow the cached config the
 * control writes through. Puts the preference back in `finally`.
 * Captures /tmp/sensei-shots/glyphs-icon.png.
 * Run: `bun run test:inspect -- glyphs`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateToScreen } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';

const storedGlyphs = async () =>
  (await (await fetch(`${DAEMON_URL}/api/config`)).json())['ui.glyphs'] as string | undefined;

/** Every settings-rail icon on screen: its class and whether it paints. */
const RAIL_ICONS = `Array.from(document.querySelectorAll('[data-component="settings-sidebar"] [data-item-icon]')).map((e) => {
  const s = getComputedStyle(e); const r = e.getBoundingClientRect();
  return { cls: e.className, painted: r.width > 0 && r.height > 0 && ((s.maskImage || s.webkitMaskImage || '') + '').includes('svg') };
})`;
const RAIL_KANJI = `Array.from(document.querySelectorAll('[data-component="settings-sidebar"] [data-item-icon-literal]')).map((e) => e.textContent)`;

type Page = {
  evaluate: (s: string) => Promise<unknown>;
  waitForSelector: (s: string, t?: number) => Promise<void>;
};

/** Close the sign-in overlay if it opened itself (an identity needing the
 *  user, e.g. #283's keychain approval): it is modal and covers the page. */
async function dismissSignIn(tauriPage: Page) {
  const state = await tauriPage.evaluate(`(function () {
    var b = document.querySelector('[data-component="sign-in-overlay"] footer button');
    if (!b) return 'absent';
    b.click(); return 'closed';
  })()`);
  console.log(`[inspect] sign-in overlay: ${state}`);
  await expect
    .poll(() => tauriPage.evaluate(`!!document.querySelector('[data-component="sign-in-overlay"]')`), {
      timeout: 5_000,
    })
    .toBe(false);
}

async function choose(tauriPage: Page, label: string) {
  await tauriPage.evaluate(
    `document.querySelector('[data-testid="pref-glyphs"] [data-select-trigger]').click()`,
  );
  await tauriPage.waitForSelector('[data-select-dropdown] [data-select-option]', 5_000);
  const picked = await tauriPage.evaluate(`(function () {
    var opts = document.querySelectorAll('[data-select-dropdown] [data-select-option]');
    for (var i = 0; i < opts.length; i++) if ((opts[i].textContent || '').trim() === ${JSON.stringify(label)}) { opts[i].click(); return 'clicked'; }
    return JSON.stringify(Array.from(opts).map(function (o) { return o.textContent; }));
  })()`);
  expect(picked, `option ${label}`).toBe('clicked');
}

test('the rails follow the glyph preference', async ({ tauriPage }) => {
  mkdirSync(DIR, { recursive: true });
  const before = await storedGlyphs();
  try {
    await navigateToScreen(tauriPage, '/settings/general', '[data-testid="pref-glyphs"]');
    await dismissSignIn(tauriPage);

    await choose(tauriPage, 'Icons');
    await tauriPage.waitForSelector('[data-component="settings-sidebar"] [data-item-icon]', 10_000);
    await expect.poll(storedGlyphs, { timeout: 10_000 }).toBe('icon');
    const icons = (await tauriPage.evaluate(RAIL_ICONS)) as { cls: string; painted: boolean }[];
    const unpainted = icons.filter((i) => !i.painted).map((i) => i.cls);
    console.log(`\n[inspect] icon mode: ${icons.length} rail icons; unpainted: ${unpainted.join(', ') || 'none'}`);
    await tauriPage.screenshot({ path: `${DIR}/glyphs-icon.png` });
    expect(icons.length).toBe(11);
    expect(icons.every((i) => /^i-solar-.+-bold-duotone$/.test(i.cls))).toBe(true);
    expect(unpainted).toEqual([]);
    expect((await tauriPage.evaluate(RAIL_KANJI)) as string[]).toEqual([]);

    await choose(tauriPage, 'Kanji');
    await tauriPage.waitForSelector('[data-component="settings-sidebar"] [data-item-icon-literal]', 10_000);
    await expect.poll(storedGlyphs, { timeout: 10_000 }).toBe('kanji');
    const kanji = (await tauriPage.evaluate(RAIL_KANJI)) as string[];
    console.log(`[inspect] kanji mode: ${kanji.join(' ')}`);
    expect(kanji).toContain('名');
  } finally {
    await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ 'ui.glyphs': before === 'icon' ? 'icon' : 'kanji' }),
    });
  }
});
