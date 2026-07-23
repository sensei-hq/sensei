<script lang="ts">
	import { page } from '$app/stores';
	import { afterNavigate, goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import ConsoleTopBar from '$lib/components/ConsoleTopBar.svelte';
	import ConsoleNav from '$lib/components/ConsoleNav.svelte';
	import { enterOrg } from '$lib/tenant';
	import type { DojoOrg } from '$lib/dojo-data';

	// The shared console shell (mockup DojoConsole): top bar + left nav wrapping
	// every console screen. The active nav section is derived from the current
	// path so each destination lights the right entry.
	let { data, children } = $props();

	// Mobile nav drawer state — on md:+ the sidebar is always visible, so this only
	// drives phone widths. Close it once navigation completes (tapping a destination).
	let navOpen = $state(false);
	afterNavigate(() => {
		navOpen = false;
	});

	const consoleHref = resolve('/(console)/console');

	// The ONE shared tenant-switch path, reused from the org picker (/orgs): the
	// top-bar switcher emits the picked org here; we persist it as the dojo_tenant
	// cookie and navigate to the console home. No second cookie/navigation path.
	function switchOrg(org: DojoOrg) {
		enterOrg(org, {
			setCookie: (cookie) => (document.cookie = cookie),
			navigate: (to) => goto(to),
			consoleHref
		});
	}

	// The pinned "Relay · you" entry: the personal home (no tenant). Landing on
	// /console with no dojo_tenant cookie renders the personal home (DJ1); we only
	// navigate here (clearing the cookie is a morning concern — the server resolves
	// membership, and a solo user has none).
	function relayHome() {
		goto(consoleHref);
	}

	// Map the current path to the active nav section id. Admin sub-routes
	// (members / identities / policies / health / audit) and the lead
	// sub-routes (engagements incl. its [id] audit child, incidents) each light
	// their own entry; everything else falls back to Overview (the console index).
	const active = $derived.by(() => {
		const path = $page.url.pathname;
		const section = path.replace(/^\/console\/?/, '').split('/')[0];
		const known = [
			'triage',
			'relay',
			'library',
			'preview',
			'teams',
			'contributions',
			'downstream',
			'members',
			'identities',
			'policies',
			'health',
			'audit',
			'engagements',
			'incidents'
		];
		return known.includes(section) ? section : 'overview';
	});
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape' && navOpen) navOpen = false;
	}}
/>

<div class="bg-paper flex h-screen w-full flex-col overflow-hidden">
	<ConsoleTopBar
		org={data.org}
		memberships={data.memberships}
		hasMembership={data.hasMembership}
		onSwitch={switchOrg}
		onRelayHome={relayHome}
		navExpanded={navOpen}
		onMenu={() => (navOpen = true)}
	/>
	<div class="flex min-h-0 flex-1">
		<ConsoleNav {active} open={navOpen} onClose={() => (navOpen = false)} />
		<main class="min-w-0 flex-1 overflow-y-auto md:overflow-hidden">
			{@render children()}
		</main>
	</div>
</div>
