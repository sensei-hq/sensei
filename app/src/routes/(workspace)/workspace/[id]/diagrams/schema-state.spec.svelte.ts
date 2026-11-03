import { describe, expect, it } from 'vitest';
import { SchemaState, type SchemaApi } from './schema-state.svelte.js';
import type { SchemaPayload, SchemaTable } from '$lib/types.js';

function table(schema: string, name: string, columns = ['id']): SchemaTable {
  return {
    schema,
    name,
    kind: 'table',
    columns: columns.map((c, i) => ({ name: c, type: 'uuid', pk: i === 0, nn: true })),
  };
}

/** Two schemas joined by one key, a view reading across them, and an enum —
 *  the shapes the live payload has. */
function payload(over: Partial<SchemaPayload> = {}): SchemaPayload {
  return {
    version: 3,
    tables: [table('app', 'customers'), table('app', 'orders', ['id', 'customer_id']), table('crm', 'accounts')],
    refs: [{ from: { s: 'app', t: 'orders', c: 'customer_id' }, to: { s: 'app', t: 'customers', c: 'id' } }],
    entities: [{ schema: 'app', name: 'order_totals', kind: 'view' }],
    deps: [{ from: { s: 'app', n: 'order_totals' }, to: { s: 'app', n: 'orders' }, kind: 'reads' }],
    enums: [{ schema: 'app', name: 'order_status', values: ['pending', 'paid'] }],
    ...over,
  };
}

const api = (res: Awaited<ReturnType<SchemaApi['tryGetWorkspaceSchema']>>): SchemaApi => ({
  tryGetWorkspaceSchema: async () => res,
});
const loaded = async (p: SchemaPayload = payload()) => {
  const s = new SchemaState(api({ ok: true, data: p }), () => 'w1');
  await s.load();
  return s;
};

// Database · ER is dbd's viewer (user, 2026-11-02): the design's Overview,
// Diagram and Entities, or one entity's page, with the sidebar on the right.
// rokkit's views draw it; this state owns the read, the tab, the selection
// and the diagram's display settings.
describe('SchemaState', () => {
  it('starts in loading, before any request has settled', () => {
    const s = new SchemaState(api({ ok: true, data: payload() }), () => 'w1');
    expect(s.view).toBe('loading');
  });

  it('a failed request is an error with the status, never the none state', async () => {
    const s = new SchemaState(api({ ok: false, error: { status: 500, message: 'Internal Server Error' } }), () => 'w1');
    await s.load();
    expect([s.view, s.failure]).toEqual(['error', 'The daemon returned 500 Internal Server Error']);
  });

  it('nothing described is the none state; enums alone are something', async () => {
    expect((await loaded(payload({ tables: [], refs: [], entities: [], deps: [], enums: [] }))).view).toBe('none');
    expect((await loaded(payload({ tables: [], refs: [], entities: [], deps: [] }))).view).toBe('design');
  });

  it('opens on the Overview, and passes the payload to the views as the design model', async () => {
    const s = await loaded();
    expect(s.tab).toBe('overview');
    expect(s.model?.enums?.map((e) => e.name)).toEqual(['order_status']);
    expect(s.model?.tables.length).toBe(3);
  });

  it('a selection opens its page — a table, an object or an enum — and none goes back to the design', async () => {
    const s = await loaded();
    expect(s.page).toBeNull();
    s.select('app.orders');
    expect(s.page).toBe('table');
    s.select('app.order_totals');
    expect(s.page).toBe('object');
    s.select('app.order_status');
    expect(s.page).toBe('enum');
    s.select(null);
    expect([s.selected, s.page]).toEqual([null, null]);
  });

  it('a selection the reloaded design no longer has clears, rather than opening on nothing', async () => {
    const s = await loaded();
    s.select('crm.accounts');
    s.setApi(api({ ok: true, data: payload({ tables: [table('app', 'customers')] }) }));
    await s.load();
    expect([s.selected, s.page]).toEqual([null, null]);
  });
});
