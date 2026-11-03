import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';
import { isSolarBoldDuotone } from './glyphs';
import { KANJI_ICONS, KIND_ICONS, iconForKanji } from './glyph-vocabulary';

// #133: sensei's kanji are a vocabulary — 連 is "connect" wherever it appears —
// so one table gives each its icon. One rule across the app and the dōjō (user,
// 2026-10-27): kanji mode shows kanji, icon mode shows icons — every mark, no
// exceptions. A kanji that is data (a workspace's own) takes its kind's icon.
const solar = new Set(
  Object.keys(JSON.parse(readFileSync('node_modules/@iconify-json/solar/icons.json', 'utf8')).icons),
);

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return sources(p);
    return /\.(svelte|ts)$/.test(p) && !p.includes('.spec.') ? [p] : [];
  });
}

/** Kanji in a kanji position: a `kanji` field or prop, a `char=` prop, or the
 *  text of a `class="kanji…"` span. */
const KANJI_SITE = /(?:kanji["']?\s*[:=]\s*["'{]?\s*["']?|class="kanji[^"]*">\s*|char=")(\p{Script=Han})/gu;

describe('KANJI_ICONS', () => {
  it('maps every kanji to an installed Solar Bold Duotone icon', () => {
    expect(Object.keys(KANJI_ICONS).length).toBeGreaterThan(100);
    for (const [k, icon] of Object.entries(KANJI_ICONS)) {
      expect(isSolarBoldDuotone(icon, solar), `${k}: ${icon}`).toBe(true);
    }
  });

  it('covers every kanji the app renders', () => {
    const missing = new Map<string, string>();
    for (const file of sources('src')) {
      for (const m of readFileSync(file, 'utf8').matchAll(KANJI_SITE)) {
        if (!KANJI_ICONS[m[1]]) missing.set(m[1], file.replace('src/', ''));
      }
    }
    expect(Object.fromEntries(missing)).toEqual({});
  });

  it('lets no mark opt out of icon mode', () => {
    const opted = sources('src').filter((f) => /icon=\{null\}/.test(readFileSync(f, 'utf8')));
    expect(opted).toEqual([]);
  });

  it('gives the assistants their icons, each kind of data kanji its icon, and an unknown kanji none', () => {
    expect(iconForKanji('連')).toBe('i-solar-link-round-bold-duotone');
    expect(iconForKanji('刀')).toBe('i-solar-cursor-bold-duotone');
    expect(iconForKanji('禅')).toBe('i-solar-code-square-bold-duotone');
    expect(iconForKanji('鬱')).toBeUndefined();
    for (const icon of Object.values(KIND_ICONS)) expect(isSolarBoldDuotone(icon, solar), icon).toBe(true);
  });
});
