<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import DojoOrgs from '$lib/components/DojoOrgs.svelte';
	import type { DojoOrg } from '$lib/dojo-data';
	import { enterOrg } from '$lib/tenant';
	import type { PageData } from './$types';

	// user + orgs are loaded server-side from the real Supabase session +
	// `dojo.memberships` (see +page.server.ts).
	let { data }: { data: PageData } = $props();

	function enter(org: DojoOrg) {
		// The one shared tenant-switch path (also used by the top-bar switcher):
		// persist the selected org as the `dojo_tenant` session cookie — read
		// server-side in `(console)/+layout.server.ts` — then navigate to the
		// console home. Side effects injected so `enterOrg` stays pure/testable.
		enterOrg(org, {
			setCookie: (cookie) => (document.cookie = cookie),
			navigate: (to) => goto(to),
			consoleHref: resolve('/(console)/console')
		});
	}
</script>

<svelte:head>
	<title>Your organizations · Dōjō</title>
</svelte:head>

<DojoOrgs user={data.user} orgs={data.orgs} onEnter={enter} />
