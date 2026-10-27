import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { isSolarBoldDuotone } from '$lib/glyphs';
import { DIAGRAM_VIEWS } from './diagrams-nav';

// #133: every diagram view carries a kanji AND a Solar Bold Duotone icon.
describe('DIAGRAM_VIEWS marks', () => {
  const solar = new Set(
    Object.keys(JSON.parse(readFileSync('node_modules/@iconify-json/solar/icons.json', 'utf8')).icons),
  );
  it('gives every view a kanji and a Solar icon', () => {
    expect(DIAGRAM_VIEWS.length).toBe(7);
    for (const v of DIAGRAM_VIEWS) {
      expect(v.kanji, v.id).toMatch(/^\p{Script=Han}$/u);
      expect(isSolarBoldDuotone(v.icon, solar), `${v.id}: ${v.icon}`).toBe(true);
    }
  });
});
