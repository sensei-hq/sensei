// The shared repository drill-in context (#532): the org from the route
// against real memberships (redirect to /you if not a member), the
// repository by id from the caller's repos and belonging to this tenant
// (else back to the org page — a drill never dead-ends on a repo that is
// not here), the branch from ?branch= when it names a tracked branch, else
// the tracked default. The diagrams page owns this logic today; the repo
// screen (and later the workspace screen) shares it instead of copying it.
import { redirect } from '@sveltejs/kit';
import { orgBySlug, type DojoOrg } from './chrome';
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
	};
	branch: string | null;
}

export async function loadRepoContext(opts: {
	slug: string;
	id: string;
	branch: string | null;
	memberships: DojoOrg[];
	accessToken: string | null;
	fetch: DojoCallOpts['fetch'];
	listRepos?: (o: DojoCallOpts) => Promise<MyRepoWire[]>;
}): Promise<RepoContext> {
	const org = orgBySlug(opts.memberships, opts.slug);
	if (!org) redirect(307, '/you');
	let repos: MyRepoWire[];
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
			last_activity_at: repo.last_activity_at ?? null
		},
		branch
	};
}
