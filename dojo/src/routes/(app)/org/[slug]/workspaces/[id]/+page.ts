import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgHref } from '$lib/nav';
import { orgBySlug } from '$lib/chrome';
import { listWorkspaces } from '$lib/client-data';
import { toKitWorkspaces } from '$lib/workspaces-map';
import { loadWorkspaceConstitution } from '$lib/workspace-constitution';
import { getWorkspaceMetrics } from '$lib/workspace-metrics-data';
import { toWorkspaceMetricsView, type WorkspaceMetricsView } from '$lib/workspace-metrics-view';
import type { KitWorkspace } from '$lib/components/kit/types';

// The org workspace-constitution preview drill-in loader (mockup ScrWorkspacePreview
// in the org context). Resolves the org from the route against real memberships
// (redirect to /you if not a member), the workspace by id from the dōjō's REAL
// workspaces (GET …/workspaces), and its daemon-resolved constitution by id — the
// same read as /you/workspaces/[id] (#106). It used to read kit fixtures keyed on
// mockup slugs, so every real org's preview redirected straight back. An unknown
// id, or a workspaces-read error, redirects to the org workspace list so the drill
// never dead-ends on a fabricated workspace.
export const load: PageLoad = async ({ parent, params, fetch }) => {
	const { memberships, accessToken } = await parent();
	const org = orgBySlug(memberships, params.slug);
	if (!org) redirect(307, '/you');
	const back = orgHref(params.slug, 'workspaces');

	let workspace: KitWorkspace | undefined;
	try {
		const rows = await listWorkspaces(org.url, { fetch, accessToken });
		workspace = toKitWorkspaces(rows).find((w) => w.id === params.id);
	} catch {
		redirect(307, back);
	}
	if (!workspace) redirect(307, back);

	const { ladder, conflicts, error } = await loadWorkspaceConstitution(workspace.id, { fetch, accessToken });

	// The workspace's metrics (WP5): per-repository charts from the WP4 read.
	// A failure is carried SEPARATELY from an empty window — "nothing sent" and
	// "could not ask" are different answers (F1, #109).
	let metricsView: WorkspaceMetricsView | null = null;
	let metricsError: string | null = null;
	try {
		metricsView = toWorkspaceMetricsView(await getWorkspaceMetrics(org.url, workspace.id, { fetch, accessToken }));
	} catch (e) {
		metricsError = e instanceof Error ? e.message : 'the request failed';
	}

	return {
		slug: params.slug,
		orgName: org.name,
		workspace,
		ladder,
		conflicts,
		constitutionError: error,
		metricsView,
		metricsError
	};
};
