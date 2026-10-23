import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { youHref } from '$lib/nav';
import { listUserWorkspaces } from '$lib/client-data';
import { toKitWorkspaces } from '$lib/workspaces-map';
import { loadWorkspaceConstitution } from '$lib/workspace-constitution';
import type { KitWorkspace } from '$lib/components/kit/types';

// The workspace-constitution drill-in loader (F4). Resolves the REAL workspace from
// the user-wide read (GET /v1/you/workspaces) by id, then reads its daemon-resolved
// constitution (GET /v1/you/workspaces/{slug}/constitution) and maps it to the kit
// ladder + conflicts the preview renders. The daemon OWNS the resolution; the dōjō
// only displays it (never re-resolves). No federated constitution yet → an empty
// ladder, and the screen shows its "resolves in your editor" state; a failed read
// is shown as an error (#106) — never a fabricated ladder. A no-membership
// viewer, an unknown id, or a workspaces-read error redirects to the list so the
// drill never dead-ends on a fabricated workspace.
export const load: PageLoad = async ({ params, parent, fetch }) => {
	const { hasMembership, accessToken } = await parent();
	if (!hasMembership) redirect(307, youHref('workspaces'));

	let workspace: KitWorkspace | undefined;
	try {
		const workspaces = toKitWorkspaces(await listUserWorkspaces({ fetch, accessToken }));
		workspace = workspaces.find((p) => p.id === params.id);
	} catch {
		redirect(307, youHref('workspaces'));
	}
	if (!workspace) redirect(307, youHref('workspaces'));

	// By id: a slug is unique only within one dōjō (#299). A failed read is shown.
	const { ladder, conflicts, error } = await loadWorkspaceConstitution(workspace.id, { fetch, accessToken });

	return { workspace, ladder, conflicts, constitutionError: error };
};
