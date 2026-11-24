// The ER canvas's input: dbd's design, widened (user, 2026-11-06).
//
// rokkit draws a design's ER half as tables and foreign keys, and says views belong
// on the dependency graph because a view is a derived projection with no keys of its
// own. The ruling since: the reader wants one canvas where a table's neighbours are
// its references and a view's are what it depends on and what depends on it. That
// needs a graph rokkit's two scopes do not build, so this adapter builds it — the
// same canonical model, from the same rows, with one field map for both halves.
import { withStubs, type DesignModel } from '@rokkit/graph/schema';
import type { GraphFields } from '@rokkit/graph';

/** dbd's entity kinds that are a projection on a table, and so belong beside it. */
const VIEW_KINDS = new Set(['view', 'materialized_view']);

/** One node handed to rokkit: a dbd row plus the two fields the canvas reads. */
export interface ErNode {
  /** `schema.name`, explicit so regrouping cannot re-key a node. */
  id: string;
  schema: string;
  name: string;
  kind: string;
  /** The design module, when dbd names one. */
  module?: string;
  /** schema · module — one key per group the canvas tints and the legend names. */
  group: string;
  columns: unknown[];
  note?: string;
  noteMd?: string;
}

/** One edge: a foreign key (`reference`) or a dependency (`dependency`), one shape. */
export interface ErEdge {
  kind: 'reference' | 'dependency';
  /** The verb a dependency carries (`reads | writes | calls | member`). */
  relation?: string;
  from: { s: string; n: string; c?: string };
  to: { s: string; n: string; c?: string };
  action?: string;
}

/**
 * One field map for the merged graph.
 *
 * Both halves land in the same row shape first (`from`/`to` with a schema, a name and
 * — on a reference — the anchored column), so `sourceRow`/`targetRow` read a path a
 * dependency simply does not carry: undefined, not a missing node.
 */
export const ER_FIELDS = {
  // Identity stays the row's own, never `${group}.${label}`: the group carries the
  // module, and re-keying on it would stop every edge resolving (SCHEMA_FIELDS' note).
  id: 'id',
  label: 'name',
  group: 'group',
  kind: 'kind',
  rows: 'columns',
  note: 'noteMd',
  rowName: 'name',
  rowType: 'type',
  rowNote: 'note',
  rowBadges: { pk: 'pk', nn: 'nn', fk: 'fk', uq: 'uq' },
  source: 'from.n',
  target: 'to.n',
  sourceGroup: 'from.s',
  targetGroup: 'to.s',
  sourceRow: 'from.c',
  targetRow: 'to.c',
  edgeKind: 'kind',
  relation: 'relation',
  action: 'action',
} satisfies GraphFields;

/** A row drawn as a card: stubs included, so a reference out of the model still lands. */
function node(row: { schema: string; name: string; kind?: string; module?: string; noteMd?: string }, kind: string): ErNode {
  const group = row.module ? `${row.schema} · ${row.module}` : row.schema;
  const out: ErNode = {
    id: `${row.schema}.${row.name}`,
    schema: row.schema,
    name: row.name,
    kind,
    group,
    columns: (row as { columns?: unknown[] }).columns ?? [],
  };
  if (row.module) out.module = row.module;
  if (row.noteMd) out.noteMd = row.noteMd;
  return out;
}

/** The merged graph: tables (and their stubs) beside the views, refs beside deps. */
export function toErGraph(model: DesignModel): { nodes: ErNode[]; edges: ErEdge[]; fields: GraphFields } {
  const tables = withStubs(model);
  const nodes = [
    ...tables.tables.map((t) => node(t, t.kind ?? 'table')),
    ...(model.entities ?? [])
      .filter((e) => VIEW_KINDS.has(e.kind))
      .map((e) => node(e, e.kind === 'materialized_view' ? 'matview' : e.kind)),
  ];
  const drawn = new Set(nodes.map((n) => n.id));

  const refs: ErEdge[] = model.refs.map((r) => ({
    kind: 'reference',
    from: { s: r.from.s, n: r.from.t, c: r.from.c },
    to: { s: r.to.s, n: r.to.t, c: r.to.c },
    ...(r.action ? { action: r.action } : {}),
  }));
  // A dependency is drawn when it touches something on the canvas. A procedure
  // calling a function is neither an entity nor a relationship here, and drawing it
  // would put two unplaced cards beside every design that has routines.
  const deps: ErEdge[] = (model.deps ?? [])
    .filter((d) => drawn.has(`${d.from.s}.${d.from.n}`) || drawn.has(`${d.to.s}.${d.to.n}`))
    .map((d) => ({
      kind: 'dependency',
      relation: d.kind,
      from: { s: d.from.s, n: d.from.n },
      to: { s: d.to.s, n: d.to.n },
    }));

  return { nodes, edges: [...refs, ...deps], fields: ER_FIELDS };
}
