// @vitest-environment jsdom
// The ER canvas (user, 2026-11-06): tables and views on one picture, a press that
// walks into a neighbourhood, and a way back. The traversal rules themselves are
// `er-graph.spec.ts`'s; this file pins what a reader does.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import type { DesignModel } from '@rokkit/graph/schema';
import ErCanvas from './ErCanvas.svelte';

const MODEL: DesignModel = {
  version: 3,
  tables: [
    { schema: 'app', name: 'orders', kind: 'table', module: 'sales', columns: [{ name: 'id', type: 'uuid', pk: true }] },
    { schema: 'app', name: 'customers', kind: 'table', columns: [{ name: 'id', type: 'uuid', pk: true }] },
  ],
  refs: [{ from: { s: 'app', t: 'orders', c: 'customer_id' }, to: { s: 'app', t: 'customers', c: 'id' } }],
  entities: [
    { schema: 'app', name: 'recent_orders', kind: 'view', module: 'reporting' },
    { schema: 'app', name: 'top_customers', kind: 'view' },
  ],
  deps: [
    { from: { s: 'app', n: 'recent_orders' }, to: { s: 'app', n: 'orders' }, kind: 'reads' },
    { from: { s: 'app', n: 'top_customers' }, to: { s: 'app', n: 'recent_orders' }, kind: 'reads' },
  ],
};

let app: Record<string, unknown> | null = null;

function render(props: { focus?: string | null; onselect?: (key: string | null) => void } = {}) {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(ErCanvas, { target, props: { model: MODEL, ...props } });
  flushSync();
  return target;
}

afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

const cards = (root: Element): string[] =>
  [...root.querySelectorAll('[data-graph-node]')].map((n) => n.getAttribute('data-graph-node') ?? '');

describe('ErCanvas — tables and views on one picture', () => {
  it('draws the tables and the views', () => {
    const root = render();
    expect(cards(root).sort()).toEqual(['app.customers', 'app.orders', 'app.recent_orders', 'app.top_customers']);
  });

  it('a press on a view asks for its neighbourhood, not a page', () => {
    const onselect = vi.fn();
    const root = render({ onselect });
    (root.querySelector('[data-graph-node="app.recent_orders"]') as HTMLButtonElement).click();
    flushSync();
    expect(onselect).toHaveBeenCalledWith('app.recent_orders');
  });

  it('the neighbourhood draws what the focus reads and what reads it', () => {
    const root = render({ focus: 'app.recent_orders' });
    const panel = root.querySelector('[data-testid="er-neighbourhood"]');
    expect(panel).not.toBeNull();
    expect(panel?.getAttribute('data-centre')).toBe('app.recent_orders');
    // Its dependency (the table it reads) and its dependent (the view reading it).
    expect(cards(root).sort()).toEqual(['app.orders', 'app.recent_orders', 'app.top_customers']);
    expect(cards(root)).not.toContain('app.customers');
  });

  it('the design button asks for the whole design back', () => {
    const onselect = vi.fn();
    const root = render({ focus: 'app.recent_orders', onselect });
    (root.querySelector('[data-testid="er-design-back"]') as HTMLButtonElement).click();
    flushSync();
    expect(onselect).toHaveBeenCalledWith(null);
  });
});
