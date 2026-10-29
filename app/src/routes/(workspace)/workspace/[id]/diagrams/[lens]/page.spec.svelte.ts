// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Page from './+page.svelte';
import { CODEMAP_HOST, CodeMapHost } from '../codemap-host.svelte.js';
import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
import type { CodeMapPort, Lens, Level0, Level1 } from '@sensei/codemap';

const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 0 },
  blocks: [{
    id: 'p/a', label: 'p/a', group: 'L0', package: 'p', files: 2, symbols: 9,
    roles: { code: 2, test: 0, docs: 0, config: 0, generated: 0 }, hub: false, weight_in: 0, weight_out: 0, folded: [],
  }],
  bands: [], budget: { drawn: 1, modules: 1, bands: 0, edges: 0 }, libraries: { count: 0, top: [] },
};
const L1: Level1 = { level: 'file', module: 'p/a', dependsOn: [], usedBy: [], columns: [] };
const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
const port = (): CodeMapPort => ({
  level0: vi.fn(() => ok(L0)), module: vi.fn(() => ok(L1)), file: vi.fn(),
  zones: vi.fn(() => ok({ points: [], meanDistance: 0, coverage: { points: 0, unplaceable: 0 } })),
  cochange: vi.fn(() => ok({ hidden: [], pairs: 0, commits: 0, minTogether: 5 })),
} as unknown as CodeMapPort);

const settle = async () => { await new Promise((r) => setTimeout(r, 0)); flushSync(); };
let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

function render(lens: Lens) {
  const data = $state({ workspaceId: 'w', lens, view: DIAGRAM_VIEWS.find((v) => v.id === lens)! });
  const host = new CodeMapHost(() => 'w', port());
  target = document.body.appendChild(document.createElement('div'));
  app = mount(Page, { target, props: { data } as never, context: new Map([[CODEMAP_HOST, host]]) });
  return { data, host };
}

describe('diagrams/[lens]', () => {
  it('draws the lens the route names, under its own header', async () => {
    const { host } = render('wheel');
    await settle();
    expect(host.state.lens).toBe('wheel');
    expect(target.querySelector('[data-testid="codemap-screen"]')?.getAttribute('data-lens')).toBe('wheel');
    expect(target.textContent).toContain('Wheel');
  });

  it('follows the route to another lens on the same map', async () => {
    const { data, host } = render('structure');
    await settle();
    const map = host.state;
    data.lens = 'cycles';
    data.view = DIAGRAM_VIEWS.find((v) => v.id === 'cycles')!;
    await settle();
    expect([host.state, host.state.lens]).toEqual([map, 'cycles']);
  });

  it('opening a module from the Wheel stays open', async () => {
    const { host } = render('wheel');
    await settle();
    await host.state.openModule(host.port, 'p/a');
    await settle();
    expect(host.state.level).toBe('files');
  });
});
