<script lang="ts">
	import type { Snippet } from 'svelte';
	import TopBar from './TopBar.svelte';
	import NavPane from './NavPane.svelte';
	import type { KitOrg, KitDojo, KitMe, KitNavGroup } from './types';

	// The one desktop shell (kit K2AppShell). context "you" | "org". The nav groups
	// and (screen-supplied) context header differ; everything else is identical.
	// TopBar sits on top, then a NavPane + scrolling main. This shell owns the
	// mobile drawer open state (TopBar's hamburger toggles it, the NavPane backdrop
	// / a nav selection closes it) so screens just pass nav + content. `children`
	// is the main-column content (a screen typically opens with a ContextHeader).
	let {
		context = 'you',
		org,
		dojos = [],
		me,
		nav = [],
		active,
		needsCount = 0,
		onpick,
		onnav,
		onneeds,
		children
	}: {
		context?: 'you' | 'org';
		org?: KitOrg;
		dojos?: KitDojo[];
		me?: KitMe;
		nav?: KitNavGroup[];
		active?: string;
		needsCount?: number;
		onpick?: (slug: string) => void;
		onnav?: (id: string) => void;
		onneeds?: () => void;
		children?: Snippet;
	} = $props();

	let navOpen = $state(false);
</script>

<div
	class="bg-paper flex h-full w-full flex-col overflow-hidden"
	data-context={context}
>
	<TopBar
		{context}
		{org}
		{dojos}
		{me}
		{needsCount}
		{onpick}
		{onneeds}
		navExpanded={navOpen}
		onmenu={() => (navOpen = true)}
	/>
	<div class="flex flex-1" style="min-height: 0">
		<NavPane
			groups={nav}
			{active}
			open={navOpen}
			{onnav}
			onclose={() => (navOpen = false)}
		/>
		<div class="flex-1 overflow-auto" style="min-width: 0">
			{#if children}{@render children()}{/if}
		</div>
	</div>
</div>
