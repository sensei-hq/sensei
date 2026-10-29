import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { isSolarBoldDuotone } from '$lib/glyphs';
import { DIAGRAM_VIEWS, DEFAULT_VIEW, lensOf, viewOf } from './diagrams-nav';

// The Diagrams list (M16/M17 mockup, user 2026-10-29): one sidebar entry per diagram.
describe('DIAGRAM_VIEWS', () => {
  const solar = new Set(
    Object.keys(JSON.parse(readFileSync('node_modules/@iconify-json/solar/icons.json', 'utf8')).icons),
  );
  it('lists the seven Code map lenses, then Database · ER, each with its mark', () => {
    expect(DIAGRAM_VIEWS.map((v) => [v.id, v.label, v.kanji])).toEqual([
      ['structure', 'Structure', '構'],
      ['wheel', 'Wheel', '輪'],
      ['zones', 'Zones', '域'],
      ['matrix', 'Dependency matrix', '格'],
      ['cycles', 'Cycles', '環'],
      ['coupling', 'Hidden coupling', '縁'],
      ['complexity', 'Complexity', '繁'],
      ['schema', 'Database · ER', '表'],
    ]);
  });
  it('gives every view a Solar icon, a hint and a question', () => {
    for (const v of DIAGRAM_VIEWS) {
      expect(isSolarBoldDuotone(v.icon, solar), `${v.id}: ${v.icon}`).toBe(true);
      expect(v.hint.length, v.id).toBeGreaterThan(0);
      expect(v.question.length, v.id).toBeGreaterThan(0);
    }
  });
  it('opens on Structure', () => {
    expect(DEFAULT_VIEW).toBe('structure');
  });
  it('maps a lens route to its Code map lens, and nothing else', () => {
    expect(lensOf('wheel')).toBe('wheel');
    expect(lensOf('schema')).toBeNull();
    expect(lensOf('layers')).toBeNull();
    expect(lensOf('neighbourhood')).toBeNull();
  });
  it('names the view a path is on', () => {
    expect(viewOf('/workspace/w/diagrams/zones')).toBe('zones');
    expect(viewOf('/workspace/w/sessions')).toBeNull();
  });
});
