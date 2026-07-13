<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import DojoOrgs from '$lib/components/DojoOrgs.svelte';
	import type { DojoOrg } from '$lib/dojo-data';
	import { TENANT_PARAM, tenantKeyOf } from '$lib/tenant';

	function enter(org: DojoOrg) {
		// The per-org console lands on the guarded group. The selected org's
		// discovery url is its tenant key; thread it to the console via a query
		// param (R9 placeholder transport — see $lib/tenant.ts for the flagged gap:
		// R6 didn't persist the selection into the session).
		const tenant = encodeURIComponent(tenantKeyOf(org));
		goto(resolve(`/(console)/console?${TENANT_PARAM}=${tenant}`));
	}
</script>

<svelte:head>
	<title>Your organizations · Dōjō</title>
</svelte:head>

<DojoOrgs onEnter={enter} />
