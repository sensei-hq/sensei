import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { isSolarBoldDuotone } from '$lib/glyphs';
import { INSTRUMENT_TABS, KIND_CHIPS, kindMark } from './instrument-marks';
import { buildPlaygroundGroups } from './playground-groups';

const solar = new Set(
  Object.keys(JSON.parse(readFileSync('node_modules/@iconify-json/solar/icons.json', 'utf8')).icons),
);

// #133: every Instruments mark carries a kanji AND a Solar Bold Duotone icon,
// except the assistants' own marks, which are identities with no Solar logo
// and stay kanji in both modes (as projects do).
describe('instrument marks', () => {
  it('gives every tab and kind chip a kanji and a Solar icon', () => {
    expect(INSTRUMENT_TABS.map((t) => t.id)).toEqual(['playground', 'replay', 'insights']);
    expect(KIND_CHIPS.map((c) => c.id)).toEqual(['all', 'action', 'query']);
    for (const e of [...INSTRUMENT_TABS, ...KIND_CHIPS]) {
      expect(e.kanji, e.id).toMatch(/^\p{Script=Han}$/u);
      expect(isSolarBoldDuotone(e.icon, solar), `${e.id}: ${e.icon}`).toBe(true);
    }
  });

  it('marks a tool by its kind', () => {
    expect(kindMark('action')).toEqual({ kanji: '作', icon: 'i-solar-bolt-bold-duotone' });
    expect(kindMark('query')).toEqual({ kanji: '問', icon: 'i-solar-magnifer-bold-duotone' });
    expect(kindMark('other')).toEqual({ kanji: '·' });
  });

  it('gives the sensei group and an unknown server an icon, and leaves the assistants kanji', () => {
    const groups = buildPlaygroundGroups(
      [],
      [
        { id: 'a', acp_family: 'claude', mcp_key: 'x', enabled: true },
        { id: 'b', acp_family: 'windsurf', mcp_key: 'y', enabled: true },
      ],
      {},
    );
    const [sensei, claude, unknown] = groups;
    expect(isSolarBoldDuotone(sensei.icon ?? '', solar)).toBe(true);
    expect(claude.kanji).toBe('書');
    expect(claude.icon).toBeUndefined();
    expect(isSolarBoldDuotone(unknown.icon ?? '', solar)).toBe(true);
  });
});
