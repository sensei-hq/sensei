// The schema payload for the dōjō's ER view (#268 slice 5): a faithful port
// of the daemon's `schema_model::assemble` — stored rows into dbd's
// SchemaModel v3 wire spelling, which the rokkit views read unchanged. The
// spelling is dbd's contract: false flags and nulls are omitted exactly as
// serde's skip rules omit them, so this output diffs equal against the
// daemon's (see the slice commit). `enum_values` rides the entity; the
// dōjō's reads always supply [] — values are not synced, only named.

export interface StoredColumn {
	name: string;
	sql_type: string | null;
	primary_key: boolean;
	not_null: boolean;
	unique_key: boolean;
	foreign_key: boolean;
	enum_typed: boolean;
	default_expr: string | null;
	note: string | null;
}

export interface StoredIndex {
	name: string | null;
	definition: string | null;
	unique: boolean;
}

export interface StoredRef {
	from_column: string | null;
	to: string;
	to_column: string | null;
	on_delete: string | null;
}

export interface StoredEntity {
	fqn: string;
	name: string;
	kind: string;
	db_schema: string | null;
	source: string | null;
	naming: string | null;
	note: string | null;
	note_md: string | null;
	columns: StoredColumn[];
	indexes: StoredIndex[];
	refs: StoredRef[];
	enum_values: string[];
}

export interface StoredDep {
	from: string;
	to: string | null;
	target_name: string | null;
	kind: string;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Obj = Record<string, any>;

function opt(obj: Obj, key: string, v: unknown): void {
	if (v !== null && v !== undefined) obj[key] = v;
}

function flag(obj: Obj, key: string, v: boolean): void {
	if (v) obj[key] = true;
}

/** Assemble the payload. An fqn naming no described object, or an
 *  occurrence kind with no dbd dep kind, throws — the reads rule both out,
 *  so meeting one means they drifted. */
export function assembleSchemaModel(entities: StoredEntity[], deps: StoredDep[]): Obj {
	const named = new Map(entities.map((e) => [e.fqn, e]));
	const object = (fqn: string): StoredEntity => {
		const e = named.get(fqn);
		if (!e) throw new Error(`schema model: ${fqn} is not an object the store describes`);
		return e;
	};

	const tables: Obj[] = [];
	const objects: Obj[] = [];
	const refs: Obj[] = [];
	const enums: Obj[] = [];
	for (const e of entities) {
		if (e.kind === 'enum') {
			const en: Obj = { schema: e.db_schema, name: e.name, values: e.enum_values };
			opt(en, 'note', e.note);
			opt(en, 'noteMd', e.note_md);
			enums.push(en);
			continue;
		}
		if (e.kind !== 'table') {
			const ob: Obj = { schema: e.db_schema, name: e.name, kind: e.kind };
			opt(ob, 'note', e.note);
			opt(ob, 'noteMd', e.note_md);
			objects.push(ob);
			continue;
		}
		const t: Obj = {
			schema: e.db_schema,
			name: e.name,
			kind: e.kind,
			columns: e.columns.map((c) => {
				const col: Obj = { name: c.name, type: c.sql_type };
				flag(col, 'pk', c.primary_key);
				flag(col, 'nn', c.not_null);
				flag(col, 'en', c.enum_typed);
				opt(col, 'def', c.default_expr);
				flag(col, 'fk', c.foreign_key);
				flag(col, 'uq', c.unique_key);
				opt(col, 'note', c.note);
				return col;
			})
		};
		opt(t, 'note', e.note);
		opt(t, 'noteMd', e.note_md);
		// Key order follows serde's field order (note/note_md before
		// columns); rebuild to keep the diff byte-stable.
		const ordered: Obj = { schema: t.schema, name: t.name, kind: t.kind };
		opt(ordered, 'note', e.note);
		opt(ordered, 'noteMd', e.note_md);
		ordered.columns = t.columns;
		if (e.indexes.length > 0) {
			ordered.indexes = e.indexes.map((i) => {
				const ix: Obj = { def: i.definition };
				flag(ix, 'unique', i.unique);
				opt(ix, 'name', i.name);
				return ix;
			});
		}
		tables.push(ordered);
		for (const r of e.refs) {
			const target = object(r.to);
			const ref: Obj = {
				from: { s: e.db_schema, t: e.name, c: r.from_column },
				to: { s: target.db_schema, t: target.name, c: r.to_column }
			};
			opt(ref, 'action', r.on_delete);
			refs.push(ref);
		}
	}

	const edges: Obj[] = [];
	for (const d of deps) {
		const from = object(d.from);
		let kind: string;
		if (d.kind === 'reads' || d.kind === 'writes' || d.kind === 'calls') kind = d.kind;
		else if (d.kind === 'type_use') kind = 'member';
		else throw new Error(`schema model: ${d.from} has a "${d.kind}" dependency, which dbd has no kind for`);
		let to: Obj;
		let unresolved = false;
		if (d.to !== null) {
			const t = object(d.to);
			to = { s: t.db_schema, n: t.name };
		} else if (d.target_name !== null) {
			// dbd's own reading of a written name: `schema.name`, or bare.
			const dot = d.target_name.lastIndexOf('.');
			to = dot >= 0 ? { s: d.target_name.slice(0, dot), n: d.target_name.slice(dot + 1) } : { s: '', n: d.target_name };
			unresolved = true;
		} else {
			throw new Error(`schema model: a dependency of ${d.from} names no target`);
		}
		const edge: Obj = { from: { s: from.db_schema, n: from.name }, to, kind };
		flag(edge, 'unresolved', unresolved);
		edges.push(edge);
	}

	return { version: 3, tables, refs, entities: objects, deps: edges, enums };
}
