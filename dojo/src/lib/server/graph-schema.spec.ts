// Unit tests for the dōjō's schema assemble (`assembleSchemaModel`, #268
// slice 5): a faithful port of the daemon's `schema_model::assemble` — the
// schema store's rows into dbd's SchemaModel v3 wire spelling, which the
// rokkit views read unchanged. Cases mirror the Rust suite; the port is
// additionally diffed against the Rust implementation.
import { describe, it, expect } from 'vitest';
import { assembleSchemaModel, type StoredEntity, type StoredDep } from './graph-schema';

const entity = (over: Partial<StoredEntity> = {}): StoredEntity => ({
	fqn: 'sql·p·shop·carts·item',
	name: 'carts',
	kind: 'table',
	db_schema: 'shop',
	source: 'dbd',
	naming: 'stated',
	note: null,
	note_md: null,
	columns: [],
	indexes: [],
	refs: [],
	enum_values: [],
	...over
});

describe('assembleSchemaModel', () => {
	it('a table assembles with dbd column/index/ref spellings', () => {
		const out = assembleSchemaModel(
			[
				entity({
					columns: [
						{ name: 'id', sql_type: 'uuid', primary_key: true, not_null: true, unique_key: false, foreign_key: false, enum_typed: false, default_expr: null, note: null },
						{ name: 'state', sql_type: 'text', primary_key: false, not_null: false, unique_key: false, foreign_key: false, enum_typed: true, default_expr: "'open'", note: 'lifecycle' }
					],
					indexes: [{ name: 'carts_state_ix', definition: '(state)', unique: false }],
					refs: [{ from_column: 'owner_id', to: 'sql·p·shop·owners·item', to_column: 'id', on_delete: 'cascade' }]
				}),
				entity({ fqn: 'sql·p·shop·owners·item', name: 'owners' })
			],
			[]
		);
		expect(out.version).toBe(3);
		expect(out.tables).toHaveLength(2);
		expect(out.tables[0]).toEqual({
			schema: 'shop',
			name: 'carts',
			kind: 'table',
			columns: [
				{ name: 'id', type: 'uuid', pk: true, nn: true },
				{ name: 'state', type: 'text', en: true, def: "'open'", note: 'lifecycle' }
			],
			indexes: [{ def: '(state)', name: 'carts_state_ix' }]
		});
		// False flags and nulls are omitted, as serde's skip rules omit them.
		expect(out.tables[0].columns[0]).not.toHaveProperty('fk');
		expect(out.tables[0]).not.toHaveProperty('checks');
		expect(out.refs).toEqual([
			{ from: { s: 'shop', t: 'carts', c: 'owner_id' }, to: { s: 'shop', t: 'owners', c: 'id' }, action: 'cascade' }
		]);
	});

	it('enums and non-table kinds sort into their lists', () => {
		const out = assembleSchemaModel(
			[
				entity({ fqn: 'sql·p·shop·state·item', name: 'state', kind: 'enum', enum_values: ['open', 'shut'] }),
				entity({ fqn: 'sql·p·shop·open_carts·item', name: 'open_carts', kind: 'view' })
			],
			[]
		);
		expect(out.tables).toEqual([]);
		expect(out.enums).toEqual([{ schema: 'shop', name: 'state', values: ['open', 'shut'] }]);
		expect(out.entities).toEqual([{ schema: 'shop', name: 'open_carts', kind: 'view' }]);
	});

	it('deps map kinds and dim unresolved written names', () => {
		const dep = (over: Partial<StoredDep>): StoredDep => ({
			from: 'sql·p·shop·open_carts·item',
			to: 'sql·p·shop·carts·item',
			target_name: null,
			kind: 'reads',
			...over
		});
		const out = assembleSchemaModel(
			[
				entity({ fqn: 'sql·p·shop·open_carts·item', name: 'open_carts', kind: 'view' }),
				entity({})
			],
			[
				dep({}),
				dep({ kind: 'type_use' }),
				dep({ to: null, target_name: 'now', kind: 'calls' }),
				dep({ to: null, target_name: 'pg.temp', kind: 'calls' })
			]
		);
		expect(out.deps).toEqual([
			{ from: { s: 'shop', n: 'open_carts' }, to: { s: 'shop', n: 'carts' }, kind: 'reads' },
			{ from: { s: 'shop', n: 'open_carts' }, to: { s: 'shop', n: 'carts' }, kind: 'member' },
			{ from: { s: 'shop', n: 'open_carts' }, to: { s: '', n: 'now' }, kind: 'calls', unresolved: true },
			{ from: { s: 'shop', n: 'open_carts' }, to: { s: 'pg', n: 'temp' }, kind: 'calls', unresolved: true }
		]);
	});

	it('an unknown fqn or kind is an error, never a stub', () => {
		expect(() =>
			assembleSchemaModel([entity({ refs: [{ from_column: 'x', to: 'sql·p·shop·ghost·item', to_column: 'id', on_delete: null }] })], [])
		).toThrow(/not an object the store describes/);
		expect(() =>
			assembleSchemaModel([entity({})], [{ from: 'sql·p·shop·carts·item', to: 'sql·p·shop·carts·item', target_name: null, kind: 'nope' }])
		).toThrow(/no kind for/);
	});
});
