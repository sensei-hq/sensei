import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { orgHref } from '$lib/nav';
import { orgBySlug } from '$lib/chrome';
import { listWorkspaces } from '$lib/client-data';
import { toKitWorkspaces } from '$lib/workspaces-map';
import { getWorkspaceDelivery } from '$lib/workspace-delivery-data';
import { toDeliveryView, type DeliveryBoardView } from '$lib/workspace-delivery-view';
import type { KitWorkspace } from '$lib/components/kit/types';

// The org workspace's Delivery page (spec tracker-delivery-metrics D5; #352): the org
// from the route against real memberships (else /you), the workspace from the
// dōjō's real workspaces (else back to the list), then the workspace's boards and
// sprints read as the caller. A failed read is carried SEPARATELY from no boards.
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

	let deliveryView: { boards: DeliveryBoardView[] } | null = null;
	let deliveryError: string | null = null;
	try {
		deliveryView = toDeliveryView(await getWorkspaceDelivery(org.url, workspace.id, { fetch, accessToken }));
	} catch (e) {
		deliveryError = e instanceof Error ? e.message : 'the request failed';
	}

	return {
		slug: params.slug,
		orgName: org.name,
		tenantKey: org.url,
		workspace,
		deliveryView,
		deliveryError
	};
};
