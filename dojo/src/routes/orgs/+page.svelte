<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import DojoOrgs from '$lib/components/DojoOrgs.svelte';
	import type { DojoOrg } from '$lib/dojo-data';
	import { TENANT_COOKIE, tenantKeyOf } from '$lib/tenant';

	function enter(org: DojoOrg) {
		// The per-org console lands on the guarded group. Persist the selected
		// org as the session tenant via the `dojo_tenant` cookie (org's discovery
		// url is its tenant key), read server-side in `(console)/+layout.server.ts`.
		// Session cookie (no expiry), path=/, SameSite=Lax; not httpOnly since it's
		// server-owned but not secret. Then navigate to the plain console route.
		const tenant = tenantKeyOf(org);
		document.cookie = `${TENANT_COOKIE}=${encodeURIComponent(tenant)}; path=/; SameSite=Lax`;
		goto(resolve('/(console)/console'));
	}
</script>

<svelte:head>
	<title>Your organizations · Dōjō</title>
</svelte:head>

<DojoOrgs onEnter={enter} />
