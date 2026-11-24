import { error, redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgBySlug } from '$lib/chrome';
import { listMyRepos } from '$lib/client-data';
import type { Lens } from '@sensei/codemap';

/** The Code map lenses plus the schema ER view, which reads its own payload. */
export type DiagramLens = Lens | 'schema';

// The org repository-diagrams loader (#268): the org from the route against
// real memberships (redirect to /you if not a member), the repository by id
// from the caller's repos and belonging to this tenant (else back to the org
// page — the drill never dead-ends on a repo that is not here), the lens
// validated (unknown is a 404, never a default lens), the branch from
// ?branch= when it names a tracked branch, else the tracked default.
const LENSES: DiagramLens[] = ['structure', 'wheel', 'zones', 'matrix', 'cycles', 'coupling', 'complexity', 'schema'];

export const load: PageLoad = async ({ parent, params, url, fetch }) => {
	const { memberships, accessToken } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');
	const lens = (params.lens ?? '') as DiagramLens;
	if (!LENSES.includes(lens)) error(404, `No diagram "${params.lens}"`);

	let repos;
	try {
		repos = await listMyRepos({ fetch, accessToken });
	} catch {
		redirect(307, `/org/${params.slug}`);
	}
	const repo = repos.find((r) => r.repository_id === params.id && r.tenant === org.url);
	if (!repo) redirect(307, `/org/${params.slug}`);

	const branches = repo.graph_branches;
	const asked = url.searchParams.get('branch');
	const branch = branches.some((b) => b.branch === asked)
		? (asked as string)
		: (branches.find((b) => b.is_default)?.branch ?? branches.map((b) => b.branch).sort()[0] ?? null);

	return {
		slug: params.slug,
		orgName: org.name,
		repo: { id: repo.repository_id, key: repo.repo_key, name: repo.name ?? repo.repo_key, branches, configurable: repo.configurable_by_me },
		lens,
		branch
	};
};
