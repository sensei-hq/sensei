<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrWorkspaceDelivery from '$lib/components/screens/ScrWorkspaceDelivery.svelte';
	import { DeliveryPageState } from '$lib/delivery-state.svelte';
	import { getWorkspaceDeliveryTickets } from '$lib/workspace-delivery-data';
	import { orgHref } from '$lib/nav';

	// A workspace's delivery — its boards' sprints and tickets (tracker-delivery-metrics
	// D5; #352). A sprint's tickets are read on demand, as the caller.
	let { data } = $props();

	const state = $derived(
		new DeliveryPageState((boardId, sprintId) =>
			getWorkspaceDeliveryTickets(data.tenantKey, data.workspace.id, boardId, { sprint: sprintId }, {
				accessToken: data.accessToken
			})
		)
	);

	function back() {
		goto(orgHref(data.slug, `workspaces/${data.workspace.id}`));
	}
</script>

<svelte:head><title>Delivery · {data.workspace.name} · {data.orgName} · Dōjō</title></svelte:head>

<ScrWorkspaceDelivery
	workspaceName={data.workspace.name}
	view={data.deliveryView}
	error={data.deliveryError}
	{state}
	onBack={back}
/>
