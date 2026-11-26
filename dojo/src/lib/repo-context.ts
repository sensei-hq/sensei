// The shared repository drill-in context (#532): the org from the route
// against real memberships (redirect to /you if not a member), the
// repository by id from the caller's repos and belonging to this tenant
// (else back to the org page — a drill never dead-ends on a repo that is
// not here), the branch from ?branch= when it names a tracked branch, else
// the tracked default. The diagrams page owns this logic today; the repo
// screen (and later the workspace screen) shares it instead of copying it.
import { redirect } from '@sveltejs/kit';
import { orgBySlug } from './chrome';
import type { DojoOrg } from './dojo-data';
import { listMyRepos, type MyRepoWire } from './client-data';
import type { DojoCallOpts } from './dojo-api';

export interface RepoContext {
	slug: string;
	orgName: string;
	/** The tenant key for tenant-scoped API reads (`organization/acme`). */
	tenantKey: string;
	repo: {
		id: string;
		key: string;
		name: string;
		branches: MyRepoWire['graph_branches'];
		configurable: boolean;
		last_activity_at: string | null;
		visibility: MyRepoWire['forge_visibility'];
	};
	branch: string | null;
}

/** The repo fields the context reads — a subset of the wire row. */
export type RepoContextRow = Pick<
	MyRepoWire,
	| 'repository_id'
	| 'repo_key'
	| 'name'
	| 'tenant'
	| 'graph_branches'
	| 'configurable_by_me'
	| 'forge_visibility'
	| 'last_activity_at'
>;

export async function loadRepoContext(opts: {
	slug: string;
	id: string;
	branch: string | null;
	memberships: DojoOrg[];
	accessToken: string | null;
	fetch: DojoCallOpts['fetch'];
	listRepos?: (o: DojoCallOpts) => Promise<RepoContextRow[]>;
}): Promise<RepoContext> {
	const org = orgBySlug(opts.memberships, opts.slug);
	if (!org) redirect(307, '/you');
	let repos: RepoContextRow[];
	try {
		repos = await (opts.listRepos ?? listMyRepos)({ fetch: opts.fetch, accessToken: opts.accessToken ?? undefined });
	} catch {
		redirect(307, `/org/${opts.slug}`);
	}
	const repo = repos.find((r) => r.repository_id === opts.id && r.tenant === org.url);
	if (!repo) redirect(307, `/org/${opts.slug}`);
	const branches = repo.graph_branches;
	const branch = branches.some((b) => b.branch === opts.branch)
		? (opts.branch as string)
		: (branches.find((b) => b.is_default)?.branch ?? branches.map((b) => b.branch).sort()[0] ?? null);
	return {
		slug: opts.slug,
		orgName: org.name,
		tenantKey: org.url,
		repo: {
			id: repo.repository_id,
			key: repo.repo_key,
			name: repo.name ?? repo.repo_key,
			branches,
			configurable: repo.configurable_by_me,
			last_activity_at: repo.last_activity_at ?? null,
			visibility: repo.forge_visibility
		},
		branch
	};
}
