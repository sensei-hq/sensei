<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';
	import TabBar from './TabBar.svelte';
	import { K2_ROLE } from './vocab';
	import type { KitOrg, KitMe, KitNavItem } from './types';

	// Mobile shell (kit K2MobileShell) — a condensed top bar (context) + scrolling
	// main + bottom tabs. context "you" | "org"; the org variant carries the accent
	// top-rule and a role · kind meta line. Presentational: tab selection emits
	// `onnav(id)`. `children` is the scrolling main content.
	let {
		context = 'you',
		org,
		me,
		tabs = [],
		active,
		onnav,
		children
	}: {
		context?: 'you' | 'org';
		org?: KitOrg;
		me?: KitMe;
		tabs?: KitNavItem[];
		active?: string;
		onnav?: (id: string) => void;
		children?: Snippet;
	} = $props();

	const isYou = $derived(context === 'you');
	const initials = $derived(
		(me?.name ?? 'You')
			.replace(/\([^)]*\)/g, ' ')
			.split(/\s+/)
			.filter((w) => /^[\p{L}\p{N}]/u.test(w))
			.slice(0, 2)
			.map((w) => w[0].toUpperCase())
			.join('')
	);
</script>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<div
		class="border-paper-edge bg-paper flex flex-shrink-0 items-center gap-3 border-b {isYou
			? ''
			: 'border-t-accent border-t-2'}"
		style="padding: 12px 16px"
	>
		<span class="kanji text-accent" style="font-size: 19px; line-height: 1"
			>{isYou ? '結' : org?.kanji}</span
		>
		<div class="flex-1" style="min-width: 0">
			<div class="text-ink truncate text-sm font-semibold" style="line-height: 1.1">
				{isYou ? 'Your work' : org?.name}
			</div>
			<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
				{isYou ? 'every dōjō' : `${K2_ROLE[org?.role ?? '']?.label ?? ''} · ${org?.kind ?? ''}`}
			</div>
		</div>
		<button
			type="button"
			title="Search"
			class="bg-paper-soft border-paper-edge flex h-[34px] w-[34px] flex-shrink-0 cursor-pointer items-center justify-center rounded-full border"
		>
			<Icon name="magnifer" size={18} toneClass="text-ink-mute" />
		</button>
		<span
			class="bg-accent-soft text-accent flex flex-shrink-0 items-center justify-center rounded-full text-xs font-semibold"
			style="width: 28px; height: 28px"
			aria-hidden="true">{initials}</span
		>
		<!-- Log out — kavach's configured logout route; full navigation so the
		     sentry handle clears the session server-side. -->
		<a
			href="/logout"
			data-sveltekit-reload
			title="Log out"
			aria-label="Log out"
			class="text-ink-soft border-paper-edge flex h-[34px] w-[34px] flex-shrink-0 items-center justify-center rounded-full border bg-transparent"
		>
			<span class="kanji text-ink-mute text-xs" aria-hidden="true">出</span>
		</a>
	</div>
	<div class="flex flex-1 flex-col overflow-auto" style="min-height: 0">
		{#if children}{@render children()}{/if}
	</div>
	<TabBar {tabs} {active} {onnav} />
</div>
