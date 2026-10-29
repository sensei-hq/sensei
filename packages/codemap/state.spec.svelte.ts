import { describe, expect, it, vi } from 'vitest';
import { CodeMapState } from './state.svelte';
import type { CodeMapPort, Level0, Level1, Level2 } from './types';

const L0 = (groupBy: 'layers' | 'folders' = 'layers'): Level0 => ({
  level: 'module', groupBy, cycles: [], coverage: { unplaced: 3 },
  blocks: [{ id: 'p/a', label: 'p/a', group: 'L0', files: 2, symbols: 9, hub: false, weight_in: 0, weight_out: 1, folded: [] }],
  bands: [], budget: { drawn: 1, modules: 1, bands: 0, edges: 1 },
});
const L1: Level1 = {
  level: 'file', module: 'p/a', dependsOn: [], usedBy: [],
  columns: [
    { role: 'code', files: [{ id: 'f1', path: 'a/x.rs', symbols: 3 }], more: 0, more_symbols: 0 },
    { role: 'test', files: [{ id: 'f2', path: 'a/tests/t.rs', symbols: 1 }], more: 0, more_symbols: 0 },
  ],
};
const L2: Level2 = {
  level: 'symbol', file: 'f1', path: 'a/x.rs', symbols: [], callers: [], callersTotal: 0, callees: [], calleesTotal: 0,
};
const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
function port(over: Partial<CodeMapPort> = {}): CodeMapPort {
  return {
    level0: vi.fn((g) => ok(L0(g))), module: vi.fn(() => ok(L1)), file: vi.fn(() => ok(L2)),
    zones: vi.fn(() => ok({ points: [], meanDistance: 0, coverage: { points: 0, unplaceable: 0 } })),
    cochange: vi.fn(() => ok({ hidden: [], pairs: 0, commits: 12, minTogether: 5 })), ...over,
  };
}

describe('CodeMapState', () => {
  it('opens on modules, and drills module → file with a breadcrumb', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    expect([s.level, s.l0?.blocks.length]).toEqual(['modules', 1]);
    await s.openModule(p, 'p/a');
    expect([s.level, s.crumbs.map((c) => c.label)]).toEqual(['files', ['workspace', 'p/a']]);
    await s.openFile(p, 'f1', 'a/x.rs');
    expect([s.level, s.crumbs.map((c) => c.label)]).toEqual(['symbols', ['workspace', 'p/a', 'a/x.rs']]);
    s.crumbTo(1);
    expect([s.level, s.crumbs.length, s.l2]).toEqual(['files', 2, null]);
    s.crumbTo(0);
    expect([s.level, s.crumbs.length]).toEqual(['modules', 1]);
  });

  it('keeps level, selection and lens when the grouping changes', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    s.select('p/a');
    s.lens = 'matrix';
    await s.setGroupBy(p, 'folders');
    expect(p.level0).toHaveBeenLastCalledWith('folders');
    expect([s.groupBy, s.selected, s.lens, s.level]).toEqual(['folders', 'p/a', 'matrix', 'modules']);
  });

  it('hides a role from the file columns when it is turned off', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.load(p);
    await s.openModule(p, 'p/a');
    expect(s.columns.map((c) => c.role)).toEqual(['code', 'test']);
    s.toggleRole('test');
    expect(s.columns.map((c) => c.role)).toEqual(['code']);
  });

  it('says why a read failed, and stays where it was', async () => {
    const s = new CodeMapState();
    const p = port({ module: vi.fn(() => Promise.resolve({ ok: false as const, error: 'Service Unavailable' })) });
    await s.load(p);
    await s.openModule(p, 'p/a');
    expect([s.level, s.error]).toEqual(['modules', 'Service Unavailable']);
  });

  it('the Wheel, Zones and Matrix return to modules with the open module selected', async () => {
    for (const lens of ['wheel', 'zones', 'matrix'] as const) {
      const s = new CodeMapState();
      const p = port();
      await s.load(p);
      await s.openModule(p, 'p/a');
      await s.openFile(p, 'f1', 'a/x.rs');
      await s.showLens(p, lens);
      expect([s.lens, s.level, s.crumbs.length, s.l1, s.l2, s.selected], lens).toEqual([lens, 'modules', 1, null, null, 'p/a']);
    }
  });

  it('Structure, Cycles, Hidden coupling and Complexity keep the level', async () => {
    for (const lens of ['structure', 'cycles', 'coupling', 'complexity'] as const) {
      const s = new CodeMapState();
      const p = port();
      await s.load(p);
      await s.openModule(p, 'p/a');
      await s.showLens(p, lens);
      expect([s.lens, s.level, s.crumbs.length], lens).toEqual([lens, 'files', 2]);
    }
  });

  it('reads zones once, when the Zones lens first opens', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.showLens(p, 'zones');
    await s.showLens(p, 'structure');
    await s.showLens(p, 'zones');
    expect([s.lens, s.zones?.meanDistance, p.zones]).toEqual(['zones', 0, p.zones]);
    expect(p.zones).toHaveBeenCalledTimes(1);
  });

  it('reads co-change once, when the Hidden coupling lens first opens', async () => {
    const s = new CodeMapState();
    const p = port();
    await s.showLens(p, 'coupling');
    await s.showLens(p, 'coupling');
    expect([s.lens, s.cochange?.commits]).toEqual(['coupling', 12]);
    expect(p.cochange).toHaveBeenCalledTimes(1);
  });

  it('drops an answer that a later request overtook', async () => {
    const s = new CodeMapState();
    let release: (v: { ok: true; data: Level1 }) => void = () => {};
    const slow = new Promise<{ ok: true; data: Level1 }>((r) => (release = r));
    const p = port({ module: vi.fn().mockReturnValueOnce(slow).mockReturnValueOnce(ok({ ...L1, module: 'p/b' })) });
    await s.load(p);
    const first = s.openModule(p, 'p/a');
    await s.openModule(p, 'p/b');
    release({ ok: true, data: L1 });
    await first;
    expect(s.l1?.module).toBe('p/b');
  });
});
