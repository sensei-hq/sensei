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
