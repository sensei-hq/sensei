// The ER canvas's graph (user, 2026-11-06): tables and views on one canvas, grouped
// by schema and module, with a table's references and a view's dependencies as the
// edges a press traverses. The pins are the four rulings, one each.
import { describe, expect, it } from 'vitest';
import { normalizeGraph } from '@rokkit/graph';
import type { DesignModel } from '@rokkit/graph/schema';
import { toErGraph } from './er-graph.js';

const MODEL: DesignModel = {
  version: 3,
  project: { name: 'sensei', db: 'postgres' },
  tables: [
    {
      schema: 'app',
      name: 'orders',
      kind: 'table',
      module: 'sales',
      columns: [
        { name: 'id', type: 'uuid', pk: true },
        { name: 'customer_id', type: 'uuid', fk: true, en: false },
      ],
    },
    { schema: 'app', name: 'customers', kind: 'table', columns: [{ name: 'id', type: 'uuid', pk: true }] },
  ],
  refs: [
    {
      from: { s: 'app', t: 'orders', c: 'customer_id' },
      to: { s: 'app', t: 'customers', c: 'id' },
      action: 'no action',
    },
  ],
  entities: [
    { schema: 'app', name: 'recent_orders', kind: 'view', module: 'reporting' },
    { schema: 'app', name: 'top_customers', kind: 'view' },
    { schema: 'app', name: 'refund', kind: 'function', module: 'ops' },
    { schema: 'app', name: 'refund_audit', kind: 'function', module: 'ops' },
  ],
  deps: [
    { from: { s: 'app', n: 'recent_orders' }, to: { s: 'app', n: 'orders' }, kind: 'reads' },
    { from: { s: 'app', n: 'top_customers' }, to: { s: 'app', n: 'recent_orders' }, kind: 'reads' },
    // A routine calling a routine: not an entity, not drawn, and its edge goes too.
    { from: { s: 'app', n: 'refund' }, to: { s: 'app', n: 'refund_audit' }, kind: 'calls' },
  ],
};

const graphOf = (model: DesignModel) => {
  const graph = toErGraph(model);
  return { ...graph, model: normalizeGraph(graph.nodes, graph.edges, graph.fields) };
};
const graph = () => graphOf(MODEL);

describe('toErGraph — one canvas, tables and views', () => {
  it('draws the views beside the tables, and leaves the routines off', () => {
    const { model } = graph();
    const ids = model.nodes.map((n) => n.id);
    expect(ids).toEqual(['app.orders', 'app.customers', 'app.recent_orders', 'app.top_customers']);
    expect(model.nodes.find((n) => n.id === 'app.recent_orders')?.kind).toBe('view');
  });

  it('a node keys on schema.name even though its group carries the module', () => {
    const { model } = graph();
    // `app · sales` is the group; the id stays the database's own key, so edges
    // resolve and a module move cannot orphan one.
    expect(model.nodes.find((n) => n.id === 'app.orders')?.group).toBe('app · sales');
    expect(model.edges.find((e) => e.kind === 'reference')?.source).toBe('app.orders');
  });

  it('groups by schema and module, and by schema alone when there is no module', () => {
    const { model } = graph();
    const groupOf = (id: string) => model.nodes.find((n) => n.id === id)?.group;
    expect(groupOf('app.recent_orders')).toBe('app · reporting');
    expect(groupOf('app.customers')).toBe('app');
  });

  it('a table’s reference is a reference edge, a view’s dep a dependency edge with its verb', () => {
    const { model } = graph();
    const ref = model.edges.find((e) => e.kind === 'reference');
    expect(ref).toMatchObject({ source: 'app.orders', target: 'app.customers', sourceRow: 'customer_id', action: 'no action' });
    const dep = model.edges.find((e) => e.relation === 'reads' && e.source === 'app.recent_orders');
    expect(dep).toMatchObject({ kind: 'dependency', target: 'app.orders' });
    // A dependency anchors no column: a view reads a table, not a column of it.
    expect(dep?.sourceRow).toBeUndefined();
  });

  it('a view ’s dependents are edges into it, which is what its neighbourhood traverses', () => {
    const { model } = graph();
    const into = model.edges.filter((e) => e.target === 'app.recent_orders').map((e) => e.source);
    expect(into).toEqual(['app.top_customers']);
  });

  it('an edge between two undrawn routines never reaches the canvas', () => {
    const { model } = graph();
    expect(model.edges.some((e) => e.relation === 'calls')).toBe(false);
  });

  it('a dependency on something the design does not carry is kept, and flagged unplaced', () => {
    const { model } = graphOf({
      ...MODEL,
      deps: [{ from: { s: 'app', n: 'recent_orders' }, to: { s: 'other', n: 'gone' }, kind: 'reads' }],
    });
    const edge = model.edges.find((e) => e.relation === 'reads');
    expect(edge?.unplaced).toBe('target');
    // The name as written at the use site, per rokkit's rule for an unplaced end.
    expect(edge?.target).toBe('gone');
  });

  it('stubs come with the tables, so a reference out of the model still lands', () => {
    const { model } = graphOf({
      ...MODEL,
      stubs: [{ schema: 'auth', name: 'users', kind: 'external', columns: [] }],
    });
    expect(model.nodes.map((n) => n.id)).toContain('auth.users');
  });
});
