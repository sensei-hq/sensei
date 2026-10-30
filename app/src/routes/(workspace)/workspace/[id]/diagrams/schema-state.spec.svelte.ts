import { describe, expect, it } from 'vitest';
import { SchemaState, objectId, type SchemaApi } from './schema-state.svelte.js';
import type { SchemaPayload, SchemaTable } from '$lib/types.js';

function table(schema: string, name: string, columns = ['id']): SchemaTable {
  return {
    schema,
    name,
    kind: 'table',
    columns: columns.map((c, i) => ({ name: c, type: 'uuid', pk: i === 0, nn: true })),
  };
}

/** Two schemas joined by one key, a view reading across them, and a call to a
 *  built-in — the shapes the live payload has. */
function payload(over: Partial<SchemaPayload> = {}): SchemaPayload {
  return {
    version: 2,
    tables: [
      table('app', 'customers'),
      table('app', 'orders', ['id', 'customer_id', 'account_id']),
      table('crm', 'accounts'),
    ],
    refs: [
      { from: { s: 'app', t: 'orders', c: 'customer_id' }, to: { s: 'app', t: 'customers', c: 'id' }, action: 'cascade' },
      { from: { s: 'app', t: 'orders', c: 'account_id' }, to: { s: 'crm', t: 'accounts', c: 'id' } },
    ],
    entities: [{ schema: 'report', name: 'order_totals', kind: 'view' }],
    deps: [
      { from: { s: 'report', n: 'order_totals' }, to: { s: 'app', n: 'orders' }, kind: 'reads' },
      { from: { s: 'report', n: 'order_totals' }, to: { s: 'crm', n: 'accounts' }, kind: 'reads' },
      { from: { s: 'report', n: 'order_totals' }, to: { s: '', n: 'count' }, kind: 'calls', unresolved: true },
    ],
    ...over,
  };
}

function okApi(data: SchemaPayload = payload()): SchemaApi {
  return { tryGetWorkspaceSchema: async () => ({ ok: true, data }) };
}

function failApi(status: number, message: string): SchemaApi {
  return { tryGetWorkspaceSchema: async () => ({ ok: false, error: { status, message } }) };
}

async function loaded(api: SchemaApi = okApi()) {
  const s = new SchemaState(api, () => 'p1');
  await s.load();
  return s;
}

describe('SchemaState', () => {
  // The sidebar (dbd's /diagram, flipped right): every entity, grouped by schema,
  // tables first, each with its kind's glyph — a rokkit List's items.
  it('lists every entity by schema for the sidebar, tables before views', async () => {
    const s = await loaded();
    expect(s.sidebar.map((g) => [g.label, g.badge, g.children.map((c) => c.label)])).toEqual([
      ['app', '2', ['customers', 'orders']],
      ['crm', '1', ['accounts']],
      ['report', '1', ['order_totals']],
    ]);
    expect(s.sidebar[2].children[0]).toMatchObject({ value: objectId('report', 'order_totals'), kind: 'view' });
  });

  it('the list narrows by entity type and by name', async () => {
    const s = await loaded();
    s.toggleKind('table');
    expect(s.sidebar.map((g) => [g.label, g.children.map((c) => c.label)])).toEqual([['report', ['order_totals']]]);
    s.toggleKind('table');
    s.query = 'ord';
    expect(s.sidebar.map((g) => [g.label, g.children.map((c) => c.label)])).toEqual([
      ['app', ['orders']], ['report', ['order_totals']],
    ]);
    expect(s.kindCounts).toEqual([{ kind: 'table', count: 3, shown: true }, { kind: 'view', count: 1, shown: true }]);
  });

  it('every schema to choose from, with its count — a picker, not a row that runs out of room', async () => {
    const s = await loaded();
    expect(s.schemaChoices.map((c) => [c.label, c.value])).toEqual([
      ['All schemas · 3', ''], ['app · 2', 'app'], ['crm · 1', 'crm'], ['report · 0', 'report'],
    ]);
  });

  it("a selected entity's diagram is it and what it is keyed or linked to", async () => {
    const s = await loaded();
    s.select(objectId('app', 'customers'));
    expect((s.entityGraph?.nodes as { id: string }[]).map((n) => n.id).sort()).toEqual([objectId('app', 'customers'), objectId('app', 'orders')].sort());
  });

  it('a view can be selected too: what it is, and what it reads', async () => {
    const s = await loaded();
    s.select(objectId('report', 'order_totals'));
    expect(s.selectedEntity).toMatchObject({ schema: 'report', name: 'order_totals', kind: 'view' });
    expect(s.selectedTable).toBeNull();
    expect(s.entityReach).toEqual({ reads: ['app.orders', 'crm.accounts'], writes: [], calls: ['count'] });
  });

  // The inspector says what a change to a table reaches (M17 Database · ER).
  it('a selected table: the tables it references, those referencing it, and what reads it', async () => {
    const s = await loaded();
    s.select(objectId('app', 'orders'));
    expect(s.reach).toEqual({
      references: ['app.customers (customer_id)', 'crm.accounts (account_id)'],
      referencedBy: [],
      dependents: ['report.order_totals reads'],
    });
    s.select(objectId('app', 'customers'));
    expect(s.reach).toEqual({ references: [], referencedBy: ['app.orders (customer_id)'], dependents: [] });
    s.select(null);
    expect(s.reach).toBeNull();
  });

  it('starts in loading, before any request has settled', () => {
    expect(new SchemaState(okApi(), () => 'p1').view).toBe('loading');
  });

  // WRONG GATE: "no database here" when the daemon failed.
  it('a failed request is an error with the status, never the none state', async () => {
    const s = await loaded(failApi(500, 'Internal Server Error'));
    expect(s.view).toBe('error');
    expect(s.failure).toContain('500');
    const down = await loaded(failApi(0, 'connection refused'));
    expect(down.failure).toContain('Could not reach the daemon');
  });

  it('nothing described is the none state, a finding in words', async () => {
    const s = await loaded(okApi(payload({ tables: [], refs: [], entities: [], deps: [] })));
    expect(s.view).toBe('none');
  });

  // Keys draws TABLES; a view has no keys and would turn the ER picture into a
  // dependency picture.
  it('the keys scope draws tables and their keys, never views', async () => {
    const s = await loaded();
    expect(s.view).toBe('graph');
    expect(s.scope).toBe('er');
    expect(s.graphInput.nodes.map((n) => (n as { id: string }).id)).toEqual([
      'app.customers',
      'app.orders',
      'crm.accounts',
    ]);
    expect(s.graphInput.edges).toHaveLength(2);
  });

  it('the dependencies scope draws every object, and keeps an unresolved dep', async () => {
    const s = await loaded();
    s.setScope('dependencies');
    const ids = s.graphInput.nodes.map((n) => (n as { id: string }).id);
    expect(ids).toContain('report.order_totals');
    expect(s.graphInput.edges).toHaveLength(3);
    expect(s.coverage).toContain('1 dependency on a built-in or undescribed object, drawn dimmed');
  });

  it('offers each schema with its table count, after All', async () => {
    const s = await loaded();
    expect(s.schemaOptions).toEqual([
      { value: null, label: 'All', tables: 3 },
      { value: 'app', label: 'app', tables: 2 },
      { value: 'crm', label: 'crm', tables: 1 },
      { value: 'report', label: 'report', tables: 0 },
    ]);
  });

  // WRONG GATE: a filtered picture with no count of what it cut.
  it('a schema filter draws only its objects and COUNTS the links it cut', async () => {
    const s = await loaded();
    s.setSchema('app');
    expect(s.graphInput.nodes.map((n) => (n as { id: string }).id)).toEqual([
      'app.customers',
      'app.orders',
    ]);
    expect(s.graphInput.edges).toHaveLength(1);
    expect(s.cut).toBe(1);
    expect(s.coverage).toContain('1 key leads out of the shown schema, not drawn');

    s.setScope('dependencies');
    s.setSchema('report');
    // Both placed reads lead into hidden schemas; the built-in call is kept —
    // its endpoint is no object in ANY schema, so nothing hid it.
    expect(s.graphInput.edges).toHaveLength(1);
    expect(s.cut).toBe(2);
  });

  it('All cuts nothing and says so by saying nothing', async () => {
    const s = await loaded();
    expect(s.cut).toBe(0);
    expect(s.coverage.join(' ')).not.toContain('leads out');
  });

  it('a schema missing from a reloaded payload falls back to All', async () => {
    let data = payload();
    const s = new SchemaState({ tryGetWorkspaceSchema: async () => ({ ok: true, data }) }, () => 'p1');
    await s.load();
    s.setSchema('crm');
    data = payload({ tables: [table('app', 'customers')], refs: [], deps: [] });
    await s.load();
    expect(s.schema).toBeNull();
    expect(s.graphInput.nodes).toHaveLength(1);
  });

  it('selecting a table exposes it for the entity panel; a stale selection clears', async () => {
    const s = await loaded();
    s.select('app.orders');
    expect(s.selectedTable?.columns.map((c) => c.name)).toEqual(['id', 'customer_id', 'account_id']);
    s.setSchema('crm');
    expect(s.selected).toBeNull();
  });

  // The first line describes the PICTURE: keys are not drawn under
  // Dependencies, and views are not drawn under Keys.
  it('counts what the picture in view shows', async () => {
    const s = await loaded();
    expect(s.coverage[0]).toBe('3 tables · 2 keys');
    s.setScope('dependencies');
    expect(s.coverage[0]).toBe('4 objects · 3 dependencies');
    s.setSchema('app');
    expect(s.coverage[0]).toBe('2 objects · 0 dependencies');
  });
});
