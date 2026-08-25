<script lang="ts">
	import AppShell from './AppShell.svelte';
	import type { KitOrg, KitDojo, KitMe, KitNavGroup, KitNavItem } from './types';

	// Test-friendly wrapper for AppShell: plain props + a fixed main-column child so
	// the spec can assert the shell composes TopBar + NavPane + the phone TabBar
	// around the content.
	let {
		context = 'you',
		org,
		dojos = [],
		me,
		nav = [],
		tabs = [],
		active,
		needsCount = 0
	}: {
		context?: 'you' | 'org';
		org?: KitOrg;
		dojos?: KitDojo[];
		me?: KitMe;
		nav?: KitNavGroup[];
		tabs?: KitNavItem[];
		active?: string;
		needsCount?: number;
	} = $props();
</script>

<AppShell {context} {org} {dojos} {me} {nav} {tabs} {active} {needsCount}>
	<main>the main column content</main>
</AppShell>
