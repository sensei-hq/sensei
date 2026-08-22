<script lang="ts">
	import type { Snippet } from 'svelte';
	import TopBar from './TopBar.svelte';
	import NavPane from './NavPane.svelte';
	import TabBar from './TabBar.svelte';
	import type { KitOrg, KitDojo, KitMe, KitNavGroup, KitNavItem } from './types';

	// The one shell (kit K2AppShell), phone through desktop. context "you" | "org".
	// The nav groups and (screen-supplied) context header differ; everything else is
	// identical. TopBar sits on top, then a NavPane + scrolling main, and below `md`
	// a bottom TabBar. This shell owns the drawer open state (TopBar's hamburger
	// toggles it, the NavPane backdrop / a nav selection closes it) so screens just
	// pass nav + content. `children` is the main-column content (a screen typically
	// opens with a ContextHeader).
	//
	// It renders at every width because its parts already adapt: TopBar's hamburger
	// is `md:hidden` and its chips/search are `hidden md:*`, and NavPane is a fixed
	// drawer that goes `md:static`. That is what lets the (app) layout mount ONE
	// shell and render `children()` once — it previously mounted a desktop shell and
	// a separate phone shell side by side, each rendering `children()`, so every
	// screen was instantiated twice.
	let {
		context = 'you',
		org,
		dojos = [],
		me,
		nav = [],
		tabs = [],
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
		/** Bottom-tab destinations, shown below `md` only. */
		tabs?: KitNavItem[];
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
	{#if tabs.length}
		<div class="md:hidden">
			<TabBar {tabs} {active} {onnav} />
		</div>
	{/if}
</div>
