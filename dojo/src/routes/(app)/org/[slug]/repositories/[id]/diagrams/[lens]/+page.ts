import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { loadRepoContext } from '$lib/repo-context';
import type { Lens } from '@sensei/codemap';

/** The Code map lenses plus the schema ER view, which reads its own payload. */
export type DiagramLens = Lens | 'schema';

// The org repository-diagrams loader (#268): the shared repo context (org,
// repo, branch) plus the validated lens (unknown is a 404, never a default).
const LENSES: DiagramLens[] = ['structure', 'wheel', 'zones', 'matrix', 'cycles', 'coupling', 'complexity', 'schema'];

export const load: PageLoad = async ({ parent, params, url, fetch }) => {
	const { memberships, accessToken } = await parent();
	const lens = (params.lens ?? '') as DiagramLens;
	if (!LENSES.includes(lens)) error(404, `No diagram "${params.lens}"`);
	const ctx = await loadRepoContext({
		slug: params.slug,
		id: params.id,
		branch: url.searchParams.get('branch'),
		memberships,
		accessToken,
		fetch
	});
	return { ...ctx, lens };
};
