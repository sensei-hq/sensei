// The receiving half of the graph push (#225; docs/spec/dojo/graph-sync.md).
//
// One request is one repository at one commit of one branch: a batch of file
// slices, and on the last batch the repository's file list (the manifest) so
// the dōjō prunes deleted files. dojo.file_graph_slices is the boundary — it
// re-checks the caller's verdict, the tracked branch and the commit order, and
// takes the tenant from the verdict. This side resolves the repo_key to ONE of
// the caller's repositories, refuses by name what it can, and checks the shape.
import { refusalStatus } from './db-refusal';
import { AdminError, type DojoClient } from './admin-data';

/** What the daemon sends. Slices are passed through: their shape is the
 *  function's to read, and an unreadable field is its to refuse. */
export interface GraphPush {
	repo_key: string;
	commit: { branch: string; sha: string; committed_at: string };
	slices: unknown[];
	/** path → content_hash for every file of the branch; present on a
	 *  repository's last batch only. Absent never prunes. */
	manifest?: Record<string, string> | null;
}

export interface GraphPushResult {
	files: number;
	refused_older: number;
	pruned: number;
}

/** PostgREST's row ceiling (`PGRST_DB_MAX_ROWS`); a read is paged under it. */
const PAGE = 1000;

const nonEmpty = (v: unknown): v is string => typeof v === 'string' && v.trim() !== '';

/**
 * The caller's one repository under `repoKey`, or a refusal:
 *   404 — not among the caller's repositories. ONE answer for "never registered"
 *         and "another tenant's": telling them apart is an existence oracle.
 *   403 — theirs, but it may not sync (unelected, unsubscribed, uncaptured).
 *   409 — registered under two of the caller's tenants; picking one would file
 *         the graph under an arbitrary governance boundary.
 */
async function myRepository(db: DojoClient, repoKey: string): Promise<string> {
	const read = await db
		.rpc('my_repositories')
		.select('repository_id, tenant_id, repo_key, sync_enabled')
		.in('repo_key', [repoKey]);
	if (read.error) throw new AdminError(500, read.error.message);
	const ours = (read.data ?? []) as { repository_id: string; sync_enabled: boolean }[];
	if (ours.length === 0) throw new AdminError(404, `unknown_repository: ${repoKey}`);
	if (!ours.some((r) => r.sync_enabled)) throw new AdminError(403, `not_permitted: ${repoKey}`);
	if (ours.length > 1) throw new AdminError(409, `ambiguous: ${repoKey} is registered under two of your tenants`);
	return ours[0].repository_id;
}

function readPush(body: unknown): GraphPush {
	const b = body as Partial<GraphPush> | null;
	const c = b?.commit as Partial<GraphPush['commit']> | undefined;
	const manifestOk =
		b?.manifest == null ||
		(typeof b.manifest === 'object' &&
			!Array.isArray(b.manifest) &&
			Object.values(b.manifest).every((h) => typeof h === 'string'));
	if (
		!b ||
		!nonEmpty(b.repo_key) ||
		!c ||
		!nonEmpty(c.branch) ||
		!nonEmpty(c.sha) ||
		!nonEmpty(c.committed_at) ||
		!Array.isArray(b.slices) ||
		!manifestOk
	) {
		throw new AdminError(
			400,
			'body must be { repo_key, commit: { branch, sha, committed_at }, slices: [...], manifest?: { path: hash } }'
		);
	}
	return b as GraphPush;
}

/** File one batch of a repository's graph slices, as the caller. */
export async function ingestGraph(db: DojoClient, body: unknown): Promise<GraphPushResult> {
	const push = readPush(body);
	const repositoryId = await myRepository(db, push.repo_key);
	const { data, error } = await db
		.rpc('file_graph_slices', {
			p_repository_id: repositoryId,
			p_commit: push.commit,
			p_slices: push.slices,
			p_manifest: push.manifest ?? null
		})
		.maybeSingle();
	if (error) throw new AdminError(refusalStatus(error), error.message);
	if (!data) throw new AdminError(500, 'file_graph_slices returned no result');
	return data as GraphPushResult;
}

/** The accepted content hash of every file the dōjō holds for one branch of
 *  one of the caller's repositories — what the daemon diffs against. */
export async function graphHashes(
	db: DojoClient,
	repoKey: string,
	branch: string
): Promise<{ files: Record<string, string> }> {
	if (!nonEmpty(repoKey) || !nonEmpty(branch)) throw new AdminError(400, 'repo_key and branch are required');
	const repositoryId = await myRepository(db, repoKey);
	const files: Record<string, string> = {};
	for (let from = 0; ; from += PAGE) {
		const { data, error } = await db
			.rpc('graph_file_hashes', { p_repository_id: repositoryId, p_branch: branch })
			.range(from, from + PAGE - 1);
		if (error) throw new AdminError(refusalStatus(error), error.message);
		const page = (data ?? []) as { path: string; content_hash: string }[];
		for (const r of page) files[r.path] = r.content_hash;
		if (page.length < PAGE) return { files };
	}
}
