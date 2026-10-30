import { describe, expect, it } from 'vitest';
import { bandTone, budgetLine, complexityTier, legendFor, searchModules, shownFiles, visibleBands } from './view';
import type { Band, Block, Level0 } from './types';

const roles = { code: 4, test: 3, docs: 1, config: 0, generated: 2 };
const block = (id: string, extra: Partial<Block> = {}): Block => ({
  id, label: id, group: 'L0', package: 'p', files: 10, symbols: 10, roles, hub: false,
  weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const band = (source: string, target: string, weight: number, direction: Band['direction']): Band => ({
  source, target, weight, kinds: {}, direction,
});

describe('visibleBands', () => {
  const bands = [band('a', 'b', 30, 'up'), band('b', 'c', 5, 'down'), band('c', 'd', 12, 'skip'), band('d', 'e', 40, 'level')];
  const inCycle = new Map([['d', 0], ['e', 0]]);
  it('keeps every band by default', () => {
    expect(visibleBands(bands, { edgeMode: 'all', minWeight: 1 }, inCycle)).toHaveLength(4);
  });
  it('keeps violations and cycles only: upward, a layer skip, or inside one cycle', () => {
    expect(visibleBands(bands, { edgeMode: 'violations', minWeight: 1 }, inCycle).map((b) => b.source)).toEqual(['a', 'c', 'd']);
  });
  it('drops bands under the minimum weight', () => {
    expect(visibleBands(bands, { edgeMode: 'all', minWeight: 25 }, inCycle).map((b) => b.source)).toEqual(['a', 'd']);
  });
});

describe('bandTone', () => {
  it('draws upward red always, and the rest neutral until colour by direction is on', () => {
    expect([bandTone('up', false), bandTone('skip', false), bandTone('down', false)]).toEqual([
      'var(--danger)', 'var(--ink-faint)', 'var(--ink-faint)',
    ]);
    expect([bandTone('skip', true), bandTone('level', true), bandTone('down', true), bandTone(null, true)]).toEqual([
      'var(--warning)', 'var(--ink-mute)', 'var(--ink-faint)', 'var(--ink-faint)',
    ]);
  });
});

describe('legendFor', () => {
  it('names what each lens draws', () => {
    expect(legendFor('structure', false).map((l) => l.label)).toEqual(['dependency', 'upward']);
    expect(legendFor('structure', true).map((l) => l.label)).toEqual(['down', 'same layer', 'upward', 'skips a layer']);
    expect(legendFor('cycles', false).map((l) => l.label)).toEqual(['in a cycle']);
    expect(legendFor('coupling', false).map((l) => l.label)).toEqual(['changes together']);
    expect(legendFor('complexity', false).map((l) => l.label)).toEqual(['over 10%', 'over 20%', 'not measured']);
  });
});

describe('complexityTier', () => {
  it('is amber over 10%, red over 20%, and says when a block was not measured', () => {
    expect(complexityTier(block('a', { complexity: { functions: 10, complex: 3 } }))).toMatchObject({ tone: 'danger', label: '30% over cx 10' });
    expect(complexityTier(block('a', { complexity: { functions: 10, complex: 1.5 } }))).toMatchObject({ tone: 'warning' });
    expect(complexityTier(block('a', { complexity: { functions: 10, complex: 1 } }))).toMatchObject({ tone: 'neutral', label: '10% over cx 10' });
    expect(complexityTier(block('a', { complexity: null }))).toMatchObject({ tone: 'unmeasured', label: 'not measured' });
  });
});

describe('shownFiles', () => {
  it('counts only the roles shown, so turning off Tests shrinks the block', () => {
    expect(shownFiles(block('a'), ['code', 'test', 'docs', 'config'])).toBe(8);
    expect(shownFiles(block('a'), ['code', 'docs', 'config'])).toBe(5);
  });
});

describe('searchModules', () => {
  const l0 = {
    blocks: [block('sensei/api'), block('fold:L1', { label: '+2 more', folded: ['sensei/tasks', 'dbd/apiary'] })],
  } as unknown as Level0;
  it('finds drawn and folded modules by name, drawn first', () => {
    expect(searchModules(l0, 'api')).toEqual(['sensei/api', 'dbd/apiary']);
    expect(searchModules(l0, 'TASKS')).toEqual(['sensei/tasks']);
    expect(searchModules(l0, ' ')).toEqual([]);
  });
});

describe('budgetLine', () => {
  it('says what is drawn of what there is', () => {
    const l0 = {
      blocks: [block('a', { symbols: 1200 }), block('b', { symbols: 30 })],
      budget: { drawn: 2, modules: 5, bands: 9, edges: 18940 }, coverage: { unplaced: 0 },
    } as unknown as Level0;
    expect(budgetLine(l0, 4)).toBe('drawing 2 of 5 modules · 4 bands for 18,940 edges · 1,230 symbols');
    expect(budgetLine({ ...l0, coverage: { unplaced: 12 } }, 4)).toContain('· 12 calls unplaced');
  });
});
