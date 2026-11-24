// Unit tests for the dōjō's schema reads (`readBranchSchema`, #268 slice 5):
// one branch's schema entities with columns, indexes and refs, plus the
// entity dependencies — the daemon's `schema_detail_in`/`schema_deps_in`
// over the synced rows. Refs resolving to no node are dropped (the daemon's
// inner join); deps read occurrence site-kinds from edge props.
import { describe, it, expect } from 'vitest';
import { readBranchSchema, type DojoClient } from './graph-reads';

function makeDb(tables: Record<string, { data?: unknown[]; error?: { message: string } | null }>) {
	let table = '';
	const b: Record<string, unknown> = {};
	b.from = (t: string) => ((table = t), b);
	b.select = () => b;
	b.eq = () => b;
	b.order = () => b;
	b.range = (from: number, to: number) => {
		const t = tables[table] ?? { data: [] };
		return Promise.resolve({ data: (t.data ?? []).slice(from, to + 1), error: t.error ?? null });
	};
	return { db: b as unknown as DojoClient };
}

const NODES = [
	{ id: 'e1', folder_path: 'db', fqn: 'sql·p·shop·carts·item', name: 'carts' },
	{ id: 'e2', folder_path: 'db', fqn: 'sql·p·shop·lines·item', name: 'lines' },
	{ id: 'n9', folder_path: 'web', fqn: 'serve', name: 'serve' }
];
const ENTITIES = [
	{ node_id: 'e1', kind: 'table', db_schema: 'shop', source: 'dbd', naming: 'stated', note: null, note_md: null },
	{ node_id: 'e2', kind: 'table', db_schema: 'shop', source: 'dbd', naming: 'stated', note: null, note_md: null }
];
const COLUMNS = [
	{ node_id: 'e1', ordinal: 1, name: 'id', sql_type: 'uuid', primary_key: true, not_null: true, unique_key: false, foreign_key: false, enum_typed: false, default_expr: null, note: null }
];
const REFS = [
	{ from_node_id: 'e2', from_column: 'cart_id', to_folder: 'db', to_fqn: 'sql·p·shop·carts·item', to_column: 'id', on_delete: 'cascade' },
	{ from_node_id: 'e2', from_column: 'ghost_id', to_folder: 'db', to_fqn: 'sql·p·shop·ghost·item', to_column: 'id', on_delete: null }
];

describe('readBranchSchema', () => {
	function db() {
		return makeDb({
			graph_nodes: { data: NODES },
			graph_schema_entities: { data: ENTITIES },
			graph_schema_columns: { data: COLUMNS },
			graph_schema_indexes: { data: [] },
			graph_schema_refs: { data: REFS },
			graph_edges: { data: [] }
		}).db;
	}

	it('entities carry their node identity with columns, indexes and resolved refs', async () => {
		const out = await readBranchSchema(db(), 'r1', 'main');
		expect(out.entities).toHaveLength(2);
		const carts = out.entities.find((e) => e.name === 'carts')!;
		expect(carts).toMatchObject({ fqn: 'sql·p·shop·carts·item', kind: 'table', db_schema: 'shop' });
		expect(carts.columns).toEqual([
			expect.objectContaining({ name: 'id', sql_type: 'uuid', primary_key: true })
		]);
		const lines = out.entities.find((e) => e.name === 'lines')!;
		// The resolving ref stays with its target fqn; the dangling one is
		// dropped, as the daemon's inner join drops it.
		expect(lines.refs).toEqual([
			{ from_column: 'cart_id', to: 'sql·p·shop·carts·item', to_column: 'id', on_delete: 'cascade' }
		]);
	});

	it('deps read occurrence site-kinds between schema entities', async () => {
		const { db } = makeDb({
			graph_nodes: { data: NODES },
			graph_schema_entities: { data: ENTITIES },
			graph_schema_columns: { data: [] },
			graph_schema_indexes: { data: [] },
			graph_schema_refs: { data: [] },
			graph_edges: {
				data: [
					{
						source_id: 'e2',
						target_id: 'e1',
						kind: 'calls',
						target_name: 'carts',
						props: { occurrences: { 'db/x.ts': [{ kind: 'reads' }, { kind: 'reads' }] } }
					},
					{
						source_id: 'n9',
						target_id: 'e1',
						kind: 'calls',
						target_name: 'carts',
						props: { occurrences: { 'web/a.ts': [{ kind: 'reads' }] } }
					}
				]
			}
		});
		const out = await readBranchSchema(db, 'r1', 'main');
		// Only deps FROM a schema entity count; kinds deduplicate per pair.
		expect(out.deps).toEqual([{ from: 'sql·p·shop·lines·item', to: 'sql·p·shop·carts·item', target_name: null, kind: 'reads' }]);
	});

	it('a branch with no schema is an honest empty', async () => {
		const { db } = makeDb({});
		expect(await readBranchSchema(db, 'r1', 'dev')).toEqual({ entities: [], deps: [] });
	});
});
