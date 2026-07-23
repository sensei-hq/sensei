<script lang="ts">
	import { page } from '$app/stores';
	import { afterNavigate } from '$app/navigation';
	import ConsoleTopBar from '$lib/components/ConsoleTopBar.svelte';
	import ConsoleNav from '$lib/components/ConsoleNav.svelte';

	// The shared maintainer-console shell (mockup DojoConsole): top bar + left nav
	// wrapping every console screen. The active nav section is derived from the
	// current path so Overview / Triage / Candidate all light the right entry.
	let { data, children } = $props();

	// Mobile nav drawer state — on md:+ the sidebar is always visible, so this only
	// drives phone widths. Close it once navigation completes (tapping a destination).
	let navOpen = $state(false);
	afterNavigate(() => {
		navOpen = false;
	});

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
	<ConsoleTopBar org={data.org} navExpanded={navOpen} onMenu={() => (navOpen = true)} />
	<div class="flex min-h-0 flex-1">
		<ConsoleNav
			{active}
			tenantKey={data.tenantKey}
			open={navOpen}
			onClose={() => (navOpen = false)}
		/>
		<main class="min-w-0 flex-1 overflow-y-auto md:overflow-hidden">
			{@render children()}
		</main>
	</div>
</div>
