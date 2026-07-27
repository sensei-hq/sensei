<script lang="ts">
	import { goto } from '$app/navigation';
	import DojoOrgs from '$lib/components/DojoOrgs.svelte';
	import type { DojoOrg } from '$lib/dojo-data';
	import { orgHref } from '$lib/nav';
	import { enterOrg } from '$lib/tenant';
	import type { PageData } from './$types';

	// user + orgs are loaded server-side from the real Supabase session +
	// `dojo.memberships` (see +page.server.ts).
	let { data }: { data: PageData } = $props();

	function enter(org: DojoOrg) {
		// The one shared tenant-switch path (also used by the dojo shell's org
		// switcher): persist the selected org as the `dojo_tenant` session cookie —
		// read server-side in the shared console loader — then navigate to the dojo
		// ORG context (`/org/{slug}`), not the old `/console`. The slug is the org
		// `id` — the SAME value `toKitDojo`/`orgBySlug` (the shell + the /org/[slug]
		// load) resolve against — so entering from /orgs lands on a valid org route.
		// Side effects injected so `enterOrg` stays pure/testable.
		enterOrg(org, {
			setCookie: (cookie) => (document.cookie = cookie),
			navigate: (to) => goto(to),
			consoleHref: orgHref(org.id)
		});
	}
</script>

<svelte:head>
	<title>Your organizations · Dōjō</title>
</svelte:head>

<DojoOrgs user={data.user} orgs={data.orgs} onEnter={enter} />
