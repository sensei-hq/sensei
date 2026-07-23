<script lang="ts">
	import MobileShell from './MobileShell.svelte';
	import type { KitOrg, KitMe, KitNavItem } from './types';

	// Test-friendly wrapper for MobileShell: plain props + a fixed scrolling child.
	let {
		context = 'you',
		org,
		me,
		tabs = [],
		active
	}: {
		context?: 'you' | 'org';
		org?: KitOrg;
		me?: KitMe;
		tabs?: KitNavItem[];
		active?: string;
	} = $props();
</script>

<MobileShell {context} {org} {me} {tabs} {active}>
	<section>the scrolling main</section>
</MobileShell>
