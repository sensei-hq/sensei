// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import CodeMap from './CodeMap.svelte';
import { CodeMapState } from './state.svelte';
import type { CodeMapPort, Level0, Level1, Level2 } from './types';

const block = (id: string, extra = {}) => ({
  id, label: id, group: 'L0', files: 3, symbols: 10, hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 4 },
  blocks: [block('p/a'), block('p/b', { group: 'L1' }), block('fold:L1', { group: 'L1', label: '+2 more', folded: ['p/c', 'p/d'] })],
  bands: [{ source: 'p/a', target: 'p/b', weight: 5, kinds: { calls: 5 }, direction: 'up' }],
  budget: { drawn: 3, modules: 4, bands: 1, edges: 7 },
};
const L1: Level1 = {
  level: 'file', module: 'p/a', dependsOn: [{ module: 'p/b', weight: 5, direction: 'up' }], usedBy: [],
  columns: [
    { role: 'code', files: [{ id: 'f1', path: 'a/x.rs', symbols: 3 }], more: 2, more_symbols: 4 },
    { role: 'test', files: [{ id: 'f2', path: 'a/tests/t.rs', symbols: 1 }], more: 0, more_symbols: 0 },
  ],
};
const L2: Level2 = {
  level: 'symbol', file: 'f1', path: 'a/x.rs', symbols: [{ id: 's1', name: 'scan', kind: 'function', line: 4 }],
  callers: [{ id: 'c1', name: 'main', file: 'm.rs', weight: 2 }], callersTotal: 9, callees: [], calleesTotal: 0,
};
const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
const port = (): CodeMapPort => ({ level0: vi.fn(() => ok(L0)), module: vi.fn(() => ok(L1)), file: vi.fn(() => ok(L2)) });

let target: HTMLElement;
let app: Record<string, unknown> | null = null;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });
const render = (p: CodeMapPort) => {
  target = document.body.appendChild(document.createElement('div'));
  const state = new CodeMapState();
  app = mount(CodeMap, { target, props: { state, port: p } });
  return state;
};
const q = (sel: string) => target.querySelector(sel);
const settle = async () => { await new Promise((r) => setTimeout(r, 0)); flushSync(); };

describe('CodeMap', () => {
  it('opens on modules with the budget, and an upward band in red', async () => {
    render(port());
    await settle();
    expect(q('[data-testid="codemap-budget"]')?.textContent?.replace(/\s+/g, ' ').trim()).toBe('drawing 3 blocks for 4 modules · 1 bands for 7 edges · 4 calls unplaced');
    expect(q('[data-testid="band-p/a>p/b"]')?.getAttribute('stroke')).toBe('var(--danger)');
    expect(q('[data-testid="codemap-level"]')?.textContent).toContain('L0 · modules');
  });

  it('drills module → files → symbols, and the breadcrumb comes back up', async () => {
    const p = port();
    render(p);
    await settle();
    (q('[data-testid="block-p/a"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    flushSync();
    (q('[data-testid="codemap-open"]') as HTMLButtonElement).click();
    await settle();
    expect(p.module).toHaveBeenCalledWith('p/a');
    expect(q('[data-testid="column-code"]')?.textContent).toContain('+2 more');
    (q('[data-testid="codemap-role-test"]') as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-testid="column-test"]')).toBeNull();
    (q('[data-testid="file-a/x.rs"]') as HTMLButtonElement).click();
    await settle();
    expect(q('[data-testid="symbols-callers"]')?.textContent).toContain('+8 more');
    expect([...target.querySelectorAll('[data-testid="codemap-crumbs"] button')].map((b) => b.textContent)).toEqual([
      'workspace', 'p/a', 'a/x.rs',
    ]);
    ([...target.querySelectorAll('[data-testid="codemap-crumbs"] button')][0] as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-testid="codemap-modules"]')).not.toBeNull();
  });

  it('a folded block lists its modules instead of opening', async () => {
    render(port());
    await settle();
    (q('[data-testid="block-fold:L1"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    flushSync();
    expect(q('[data-testid="codemap-inspector"]')?.textContent).toContain('p/c');
    expect(q('[data-testid="codemap-open"]')).toBeNull();
  });

  it('says why the map could not be read', async () => {
    render({ ...port(), level0: vi.fn(() => Promise.resolve({ ok: false as const, error: 'Service Unavailable' })) });
    await settle();
    expect(q('[data-testid="codemap-error"]')?.textContent).toContain('Service Unavailable');
  });
});
