// @vitest-environment jsdom
// The side panel on its own (D87 §9, N2): a host that lays the panel out itself
// renders `CodeMapSide` in its own slot (the diagrams put it in the ScreenShell's
// sidebar), and gets exactly what CodeMap frames beside its canvas — one reader
// of the state, so a host's slot and the panel cannot drift apart.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import CodeMap from './CodeMap.svelte';
import CodeMapSide from './CodeMapSide.svelte';
import { CodeMapState } from './state.svelte';
import type { CodeMapPort, Level0 } from './types';

const roles = { code: 2, test: 1, docs: 0, config: 0, generated: 0 };
const block = (id: string, extra = {}) => ({
  id, label: id, group: 'L0', package: 'p', files: 3, symbols: 10, roles, hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 0 },
  blocks: [block('p/a'), block('p/b', { group: 'L1' })],
  bands: [{ source: 'p/a', target: 'p/b', weight: 5, kinds: { calls: 5 }, direction: 'up' }],
  budget: { drawn: 2, modules: 2, bands: 1, edges: 5 },
  libraries: { count: 0, top: [] },
};
const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
const port = (): CodeMapPort => ({
  level0: vi.fn(() => ok(L0)),
  module: vi.fn(() => ok({ level: 'file', module: 'p/a', dependsOn: [], usedBy: [], columns: [] })),
  file: vi.fn(),
  zones: vi.fn(() => ok({ meanDistance: 0, points: [], coverage: { points: 0, unplaceable: 0 } })),
  cochange: vi.fn(() => ok({ hidden: [], pairs: 0, commits: 0, minTogether: 5 })),
} as unknown as CodeMapPort);

let target: HTMLElement;
let app: Record<string, unknown> | null = null;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });
const q = (sel: string) => target.querySelector(sel);
const text = (sel: string) => (q(sel)?.textContent ?? '').replace(/\s+/g, ' ').trim();
const settle = async () => { for (let i = 0; i < 2; i++) await new Promise((r) => setTimeout(r, 0)); flushSync(); };

/** The panel inside CodeMap's own frame, for the same state. */
async function framed(p: CodeMapPort, state: CodeMapState) {
  target = document.body.appendChild(document.createElement('div'));
  app = mount(CodeMap, { target, props: { state, port: p } });
  await settle();
  return text('[data-testid="codemap-side"]');
}

describe('CodeMapSide — the panel a host lays out itself', () => {
  it('draws the same panel CodeMap frames beside the canvas', async () => {
    const p = port();
    const framedState = new CodeMapState();
    const expected = await framed(p, framedState);
    unmount(app!);
    app = null;
    target.remove();
    expect(expected).toContain('1 upward dependency');

    const state = new CodeMapState();
    await state.load(p);
    target = document.body.appendChild(document.createElement('div'));
    app = mount(CodeMapSide, { target, props: { state, port: p, label: 'Structure' } });
    await settle();
    expect(text('[data-testid="codemap-takeaways"]')).toContain('1 upward dependency');
    // Content only: the host owns the frame (the shell's sidebar scrolls it).
    expect(q('[data-testid="codemap-side"]')).toBeNull();
  });

  it('follows the state: opening a module shows its inspector', async () => {
    const p = port();
    const state = new CodeMapState();
    await state.load(p);
    target = document.body.appendChild(document.createElement('div'));
    app = mount(CodeMapSide, { target, props: { state, port: p, label: 'Structure' } });
    await settle();
    await state.openModule(p, 'p/a');
    await settle();
    expect(q('[data-testid="codemap-inspector"]')).not.toBeNull();
    expect(q('[data-testid="codemap-takeaways"]')).toBeNull();
  });
});
