// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Page from './+page.svelte';
import { CODEMAP_HOST, CodeMapHost } from '../codemap-host.svelte.js';
import type { CodeMapPort } from '@sensei/codemap';

// The user (2026-11-02): Database · ER is dbd's viewer — the design's Overview,
// Diagram and Entities, one entity's page on selection — with its sidebar on the
// right. rokkit's @rokkit/graph/schema views draw it.

const PAYLOAD = {
  version: 3,
  tables: [
    { schema: 'app', name: 'customers', kind: 'table', columns: [{ name: 'id', type: 'uuid', pk: true, nn: true }] },
    { schema: 'app', name: 'orders', kind: 'table', columns: [{ name: 'id', type: 'uuid', pk: true, nn: true }, { name: 'customer_id', type: 'uuid', nn: true, fk: true }] },
  ],
  refs: [{ from: { s: 'app', t: 'orders', c: 'customer_id' }, to: { s: 'app', t: 'customers', c: 'id' } }],
  entities: [{ schema: 'app', name: 'order_totals', kind: 'view' }],
  deps: [{ from: { s: 'app', n: 'order_totals' }, to: { s: 'app', n: 'orders' }, kind: 'reads' }],
  enums: [{ schema: 'app', name: 'order_status', values: ['pending', 'paid'] }],
};

const reply = (body: unknown, status = 200) =>
  ({ ok: status < 400, status, statusText: status < 400 ? 'OK' : 'Internal Server Error', json: async () => body }) as Response;

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); vi.unstubAllGlobals(); });

const settle = async () => { for (let i = 0; i < 4; i++) await new Promise((r) => setTimeout(r, 0)); flushSync(); };

function render(body: unknown = PAYLOAD, status = 200) {
  vi.stubGlobal('fetch', vi.fn(async () => reply(body, status)));
  const port = { level0: vi.fn(), module: vi.fn(), file: vi.fn(), zones: vi.fn(), cochange: vi.fn(), complexity: vi.fn() } as unknown as CodeMapPort;
  target = document.body.appendChild(document.createElement('div'));
  const data = { workspaceId: 'w1', workspace: { id: 'w1', name: 'sensei', repos_count: 1 } };
  app = mount(Page, { target, props: { data } as never, context: new Map([[CODEMAP_HOST, new CodeMapHost(() => 'w1', port)]]) });
  flushSync();
}
const q = (sel: string) => target.querySelector(sel);

describe('Database · ER', () => {
  it('opens on the design’s Overview, under dbd’s tabs, with the sidebar on the right', async () => {
    render();
    await settle();
    expect(q('[data-graph-design-header]')?.textContent).toContain('sensei');
    expect(q('[data-graph-design-overview]')).not.toBeNull();
    const layout = q('[data-testid="schema-layout"]') as HTMLElement;
    const [main, side] = [...layout.children] as HTMLElement[];
    expect(main.querySelector('[data-graph-design-header]')).not.toBeNull();
    expect(side.matches('[data-testid="schema-sidebar"]')).toBe(true);
    expect(side.querySelector('[data-graph-design-sidebar]')).not.toBeNull();
  });

  it('its counts are the header’s stat cards, not the Overview’s, and the sidebar has no design row', async () => {
    render();
    await settle();
    const cards = [...target.querySelectorAll('[data-testid="diagram-stats"] [data-stat-card]')].map(
      (c) => `${c.querySelector('[data-stat-card-label]')?.textContent} ${c.querySelector('[data-stat-card-value]')?.textContent}`,
    );
    expect(cards).toEqual(['Schema 1', 'Tables 2', 'View 1', 'Enum 1', 'Reference 1']);
    expect(q('[data-testid="diagram-budget"]')?.textContent).toBe('schema model v3');
    expect(q('[data-graph-section="counts"]')).toBeNull();
    expect(q('[data-testid="schema-sidebar"] [data-graph-design-home]')).toBeNull();
  });

  it('a table, an object and an enum each open their own page; back returns to the design', async () => {
    render();
    await settle();
    for (const [name, page] of [['orders', 'table'], ['order_totals', 'object'], ['order_status', 'enum']]) {
      const item = [...target.querySelectorAll('[data-testid="schema-sidebar"] [data-tree-item-content]')]
        .find((n) => n.textContent?.trim() === name) as HTMLElement;
      expect(item, name).toBeTruthy();
      item.click();
      await settle();
      expect(q('[data-testid="schema-page"]')?.getAttribute('data-page'), name).toBe(page);
    }
    expect(q('[data-testid="schema-page"]')?.textContent).toContain('pending');
    (q('[data-testid="schema-back"]') as HTMLButtonElement).click();
    await settle();
    expect(q('[data-testid="schema-page"]')).toBeNull();
    expect(q('[data-graph-design-header]')).not.toBeNull();
  });

  it('a read that failed says so, with the status, and can be retried', async () => {
    render({}, 500);
    await settle();
    expect(q('[data-testid="schema-error"]')?.textContent).toContain('500');
    expect(q('[data-testid="schema-retry"]')).not.toBeNull();
  });
});
