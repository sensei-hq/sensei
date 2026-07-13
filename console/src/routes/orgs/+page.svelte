<script lang="ts">
	import { goto } from '$app/navigation';
	import DojoOrgs from '$lib/components/DojoOrgs.svelte';
	import type { DojoOrg } from '$lib/dojo-data';

	function enter(_org: DojoOrg) {
		// The per-org console lands on the guarded group. R9–R11 wire the actual
		// console screens; for R6 this is the entry point behind the auth guard.
		goto('/console');
	}
</script>

<svelte:head>
	<title>Your organizations · Dōjō</title>
</svelte:head>

<DojoOrgs onEnter={enter} />
