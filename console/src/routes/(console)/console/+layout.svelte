<script lang="ts">
	import { page } from '$app/stores';
	import ConsoleTopBar from '$lib/components/ConsoleTopBar.svelte';
	import ConsoleNav from '$lib/components/ConsoleNav.svelte';

	// The shared maintainer-console shell (mockup DojoConsole): top bar + left nav
	// wrapping every console screen. The active nav section is derived from the
	// current path so Overview / Triage / Candidate all light the right entry.
	let { data, children } = $props();

	const active = $derived.by(() => {
		const path = $page.url.pathname;
		return path.startsWith('/console/triage') ? 'triage' : 'overview';
	});
</script>

<div class="bg-paper flex h-screen w-full flex-col overflow-hidden">
	<ConsoleTopBar org={data.org} />
	<div class="flex min-h-0 flex-1">
		<ConsoleNav {active} tenantKey={data.tenantKey} />
		<main class="min-w-0 flex-1 overflow-hidden">
			{@render children()}
		</main>
	</div>
</div>
