// One tracked branch's shared code graph, as the dōjō's diagram screens read
// it (#268). Files, nodes and edges for (repository_id, branch) under the
// tables' RLS (dojo.can_read_repository — a non-member reads nothing, which
// is indistinguishable from an unsynced branch and stays that way).
//
// Edge targets are (folder, fqn) KEYS resolved here, against the same
// branch's nodes: a key with no node stays on the edge as an external,
// never dropped. Fails closed — a query error surfaces (AdminError),
// never []-as-success; a genuinely empty branch is the honest empty.
import { AdminError, type DojoClient } from './admin-data';
import { readAllPages, graphBranchesByRepo } from './repositories';

export { AdminError };
export type { DojoClient };

export interface GraphFile {
	path: string;
	content_hash: string;
	commit_sha: string;
	committed_at: string;
}

export interface GraphNode {
	id: string;
	file_path: string | null;
	folder_path: string;
	fqn: string;
	kind: string;
	name: string;
	parent_fqn: string | null;
	line_start: number | null;
	line_end: number | null;
	is_exported: boolean;
	is_test: boolean;
	signature: string | null;
	language: string | null;
	community_id: number | null;
	tags: string[];
}

export interface GraphEdge {
	id: number;
	source_id: string;
	kind: string;
	target_folder: string | null;
	target_fqn: string | null;
	target_name: string | null;
	confidence: string | null;
	resolved_via: string | null;
	unresolved_reason: string | null;
	/** The resolved target node, or null when it is external (kept, named). */
	target_id: string | null;
	external: boolean;
}

export interface BranchGraph {
	files: GraphFile[];
	nodes: GraphNode[];
	edges: GraphEdge[];
}

const FILE_COLS = 'path, content_hash, commit_sha, committed_at';
const NODE_COLS =
	'id, file_path, folder_path, fqn, kind, name, parent_fqn, line_start, line_end, is_exported, is_test, signature, language, community_id, tags';
const EDGE_COLS =
	'id, source_id, kind, target_folder, target_fqn, target_name, confidence, resolved_via, unresolved_reason';

async function readTable<T>(db: DojoClient, table: string, cols: string, repoId: string, branch: string, orderCol: string): Promise<T[]> {
	return readAllPages<T>((from, to) =>
		db.from(table).select(cols).eq('repository_id', repoId).eq('branch', branch).order(orderCol).range(from, to)
	);
}

/** Schema tables hang off nodes and carry no branch: scoped by repository,
 *  filtered to the branch's node set by the caller. */
async function readRepoTable<T>(db: DojoClient, table: string, cols: string, repoId: string, orderCol: string): Promise<T[]> {
	return readAllPages<T>((from, to) => db.from(table).select(cols).eq('repository_id', repoId).order(orderCol).range(from, to));
}

export async function readBranchGraph(db: DojoClient, repoId: string, branch: string): Promise<BranchGraph> {
	const [files, nodes, edges] = await Promise.all([
		readTable<GraphFile>(db, 'graph_files', FILE_COLS, repoId, branch, 'path'),
		readTable<GraphNode>(db, 'graph_nodes', NODE_COLS, repoId, branch, 'folder_path'),
		readTable<Omit<GraphEdge, 'target_id' | 'external'>>(db, 'graph_edges', EDGE_COLS, repoId, branch, 'id')
	]);
	const byKey = new Map<string, string>();
	for (const n of nodes) byKey.set(`${n.folder_path}\u0000${n.fqn}`, n.id);
	return {
		files,
		nodes,
		edges: edges.map((e) => {
			const key = e.target_folder !== null && e.target_fqn !== null ? `${e.target_folder}\u0000${e.target_fqn}` : null;
			const target_id = key !== null ? (byKey.get(key) ?? null) : null;
			return { ...e, target_id, external: target_id === null };
		})
	};
}

/**
 * The caller's one repository id under `repoKey`, or a refusal:
 *   404 — not among the caller's repositories. ONE answer for "never
 *         registered" and "another tenant's": telling them apart is an
 *         existence oracle.
 *   409 — registered under two of the caller's tenants; picking one would
 *         read a graph under an arbitrary governance boundary.
 * Deliberately NOT sync-gated (unlike the ingest's resolver): a member who
 * reads but never synced still sees diagrams, and the graph tables' RLS —
 * not this function — decides what the read returns.
 */
export async function readableRepository(db: DojoClient, repoKey: string): Promise<string> {
	const read = await db.rpc('my_repositories').select('repository_id, tenant_id, repo_key, sync_enabled').in('repo_key', [repoKey]);
	if (read.error) throw new AdminError(500, read.error.message);
	const ours = (read.data ?? []) as { repository_id: string }[];
	if (ours.length === 0) throw new AdminError(404, `unknown_repository: ${repoKey}`);
	if (ours.length > 1) throw new AdminError(409, `ambiguous: ${repoKey} is registered under two of your tenants`);
	return ours[0].repository_id;
}

export interface SchemaColumn {
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

export interface SchemaIndex {
	name: string | null;
	definition: string | null;
	unique: boolean;
}

export interface SchemaRef {
	from_column: string | null;
	to: string;
	to_column: string | null;
	on_delete: string | null;
}

export interface SchemaEntity {
	fqn: string;
	name: string;
	kind: string;
	db_schema: string | null;
	source: string | null;
	naming: string | null;
	note: string | null;
	note_md: string | null;
	columns: SchemaColumn[];
	indexes: SchemaIndex[];
	refs: SchemaRef[];
}

export interface SchemaDep {
	from: string;
	to: string | null;
	target_name: string | null;
	kind: string;
}

const SCHEMA_DEP_KINDS = new Set(['reads', 'writes', 'calls', 'type_use']);

interface SchemaNodeKey {
	id: string;
	folder_path: string;
	fqn: string;
	name: string;
	language?: string | null;
}

/** One branch's schema: entities with columns, indexes and refs, plus the
 *  entity dependencies — the daemon's `schema_detail_in`/`schema_deps_in`
 *  over the synced rows. Schema tables hang off nodes (no branch of their
 *  own), so rows filter to the branch's node set in memory. A ref whose
 *  key resolves to no node is dropped, as the daemon's inner join drops it.
 *  Deps read occurrence site-kinds from edge props between schema entities.
 *  `enum_values` is not synced, so enums assemble without values (named). */
export async function readBranchSchema(
	db: DojoClient,
	repoId: string,
	branch: string
): Promise<{ entities: SchemaEntity[]; deps: SchemaDep[] }> {
	const [nodes, entities, columns, indexes, refs, edges] = await Promise.all([
		readTable<SchemaNodeKey>(db, 'graph_nodes', 'id, folder_path, fqn, name, language', repoId, branch, 'fqn'),
		readRepoTable<Record<string, unknown>>(db, 'graph_schema_entities', 'node_id, kind, db_schema, source, naming, note, note_md', repoId, 'node_id'),
		readRepoTable<Record<string, unknown>>(
			db,
			'graph_schema_columns',
			'node_id, ordinal, name, sql_type, primary_key, not_null, unique_key, foreign_key, enum_typed, default_expr, note',
			repoId,
			'node_id'
		),
		readRepoTable<Record<string, unknown>>(db, 'graph_schema_indexes', 'node_id, name, definition, unique_index', repoId, 'node_id'),
		readRepoTable<Record<string, unknown>>(
			db,
			'graph_schema_refs',
			'from_node_id, from_column, to_folder, to_fqn, to_column, on_delete',
			repoId,
			'from_node_id'
		),
		readTable<{ source_id: string; target_id: string | null; target_name: string | null; props: unknown }>(
			db,
			'graph_edges',
			'source_id, target_id, target_name, props',
			repoId,
			branch,
			'id'
		)
	]);
	const byId = new Map(nodes.map((n) => [n.id, n]));
	const byKey = new Map(nodes.map((n) => [`${n.folder_path}\u0000${n.fqn}`, n]));
	const inBranch = new Set(nodes.map((n) => n.id));
	const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);
	const bool = (v: unknown): boolean => v === true;

	const colsByNode = new Map<string, SchemaColumn[]>();
	for (const c of columns) {
		const nodeId = str(c.node_id);
		if (nodeId === null || !inBranch.has(nodeId)) continue;
		const list = colsByNode.get(nodeId) ?? [];
		list.push({
			name: str(c.name) ?? '',
			sql_type: str(c.sql_type),
			primary_key: bool(c.primary_key),
			not_null: bool(c.not_null),
			unique_key: bool(c.unique_key),
			foreign_key: bool(c.foreign_key),
			enum_typed: bool(c.enum_typed),
			default_expr: str(c.default_expr),
			note: str(c.note)
		});
		colsByNode.set(nodeId, list);
	}
	const idxByNode = new Map<string, SchemaIndex[]>();
	for (const i of indexes) {
		const nodeId = str(i.node_id);
		if (nodeId === null || !inBranch.has(nodeId)) continue;
		const list = idxByNode.get(nodeId) ?? [];
		list.push({ name: str(i.name), definition: str(i.definition), unique: bool(i.unique_index) });
		idxByNode.set(nodeId, list);
	}
	const refsByNode = new Map<string, SchemaRef[]>();
	for (const r of refs) {
		const fromId = str(r.from_node_id);
		if (fromId === null || !inBranch.has(fromId)) continue;
		const folder = str(r.to_folder);
		const fqn = str(r.to_fqn);
		const target = folder !== null && fqn !== null ? byKey.get(`${folder}\u0000${fqn}`) : undefined;
		if (!target) continue;
		const list = refsByNode.get(fromId) ?? [];
		list.push({ from_column: str(r.from_column), to: target.fqn, to_column: str(r.to_column), on_delete: str(r.on_delete) });
		refsByNode.set(fromId, list);
	}
	const entityNodeIds = new Set<string>();
	const out: SchemaEntity[] = [];
	for (const e of entities) {
		const nodeId = str(e.node_id);
		const node = nodeId !== null ? byId.get(nodeId) : undefined;
		if (!node) continue;
		entityNodeIds.add(node.id);
		out.push({
			fqn: node.fqn,
			name: node.name,
			kind: str(e.kind) ?? '',
			db_schema: str(e.db_schema),
			source: str(e.source),
			naming: str(e.naming),
			note: str(e.note),
			note_md: str(e.note_md),
			columns: colsByNode.get(node.id) ?? [],
			indexes: idxByNode.get(node.id) ?? [],
			refs: refsByNode.get(node.id) ?? []
		});
	}
	out.sort((a, b) => (a.fqn < b.fqn ? -1 : a.fqn > b.fqn ? 1 : 0));

	const seenDeps = new Set<string>();
	const deps: SchemaDep[] = [];
	for (const e of edges) {
		const s = byId.get(e.source_id);
		if (!s || !entityNodeIds.has(s.id)) continue;
		const sites = occurrences(e.props);
		for (const kind of sites) {
			if (!SCHEMA_DEP_KINDS.has(kind)) continue;
			const t = e.target_id !== null ? byId.get(e.target_id) : undefined;
			let to: string | null = null;
			let target_name: string | null = null;
			if (t && entityNodeIds.has(t.id)) {
				to = t.fqn;
			} else if (!t) {
				target_name = e.target_name;
			} else if (t.language === 'sql') {
				const segs = t.fqn.split('·');
				target_name = `${segs[2] ?? ''}.${t.name}`;
			} else {
				target_name = t.name;
			}
			if (to === null && target_name === null) continue;
			const key = `${s.fqn}\u0000${to ?? ''}\u0000${target_name ?? ''}\u0000${kind}`;
			if (seenDeps.has(key)) continue;
			seenDeps.add(key);
			deps.push({ from: s.fqn, to, target_name, kind });
		}
	}
	deps.sort((a, b) =>
		a.from < b.from ? -1 : a.from > b.from ? 1 : (a.to ?? '') < (b.to ?? '') ? -1 : (a.to ?? '') > (b.to ?? '') ? 1 : 0
	);
	return { entities: out, deps };
}

/** The occurrence site-kinds riding an edge's props (`{file: [{kind}]}`). */
function occurrences(props: unknown): string[] {
	if (typeof props !== 'object' || props === null) return [];
	const occ = (props as Record<string, unknown>).occurrences;
	if (typeof occ !== 'object' || occ === null) return [];
	const kinds: string[] = [];
	for (const sites of Object.values(occ)) {
		if (!Array.isArray(sites)) continue;
		for (const s of sites) {
			const k = (s as Record<string, unknown>)?.kind;
			if (typeof k === 'string') kinds.push(k);
		}
	}
	return kinds;
}

export interface GraphBranch {
	branch: string;
	is_default: boolean;
}

/** The repository's tracked branches with the flagged default — null when
 *  none is flagged, which the screen (not this read) resolves visibly. The
 *  same reader the sync plan uses, so the screen and the daemon cannot
 *  disagree on what is tracked. */
export async function listBranches(db: DojoClient, repoId: string): Promise<{ branches: GraphBranch[]; def: string | null }> {
	const branches = (await graphBranchesByRepo(db)).get(repoId) ?? [];
	return { branches, def: branches.find((b) => b.is_default)?.branch ?? null };
}
