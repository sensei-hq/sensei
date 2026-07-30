<script lang="ts">
	import { goto } from '$app/navigation';
	import KanjiToken from './KanjiToken.svelte';
	import { kindTone } from './vocab';
	import { K2_ROLE } from './vocab';
	import type { KitOrg, KitDojo } from './types';

	// Org-switcher popover (kit K2OrgSwitcher). `context` is "you" (personal) or an
	// org slug is active. Lists a pinned "Your work" home + every membership.
	// Purely presentational: opening/closing is local click state; picking emits
	// `onpick(slug)` where "you" is the personal home. A transparent backdrop
	// closes it (click-outside), matching the shipped ConsoleTopBar switcher.
	let {
		context = 'you',
		org,
		dojos = [],
		onpick
	}: {
		context?: 'you' | 'org';
		org?: KitOrg;
		dojos?: KitDojo[];
		onpick?: (slug: string) => void;
	} = $props();

	let open = $state(false);
	const isYou = $derived(context === 'you');
	const triggerTone = $derived(isYou ? 'text-accent' : kindTone(org?.kind).text);

	function pick(slug: string) {
		open = false;
		onpick?.(slug);
	}

	// "All organizations" / "Create or join" both leave the switcher for the org
	// index (/orgs) — the picker where a dōjō is browsed, joined, or created.
	function goOrgs() {
		open = false;
		goto('/orgs');
	}
</script>

<div class="relative">
	<button
		type="button"
		aria-haspopup="menu"
		aria-expanded={open}
		onclick={() => (open = !open)}
		onkeydown={(e) => {
			if (e.key === 'Escape' && open) {
				e.preventDefault();
				open = false;
			}
		}}
		class="bg-paper-soft inline-flex cursor-pointer items-center gap-2 rounded border {open
			? 'border-accent'
			: 'border-paper-edge'}"
		style="padding: 4px 12px; min-height: 32px"
	>
		<span class="kanji text-sm {triggerTone}">{isYou ? '携' : org?.kanji}</span>
		<span class="text-ink text-sm">{isYou ? 'Your work' : org?.name}</span>
		{#if !isYou && org}
			<span class="mono text-ink-faint text-xs uppercase" style="letter-spacing: 0.08em"
				>{org.kind}</span
			>
		{/if}
		<span class="text-ink-mute text-xs" aria-hidden="true">▾</span>
	</button>

	{#if open}
		<button
			type="button"
			tabindex="-1"
			aria-hidden="true"
			class="fixed inset-0 z-40 cursor-default border-none bg-transparent"
			onclick={() => (open = false)}
		></button>
		<div
			role="menu"
			aria-label="Switch context"
			class="bg-paper border-paper-edge absolute left-0 z-50 overflow-hidden rounded-lg border shadow-lg"
			style="top: calc(100% + 6px); width: 320px"
		>
			<div class="border-paper-edge flex items-center gap-2 border-b" style="padding: 8px 12px">
				<KanjiToken char="探" size="sm" toneClass="text-ink-mute" />
				<span class="text-ink-faint flex-1 text-sm">Switch context…</span>
				<span
					class="mono text-ink-faint bg-paper-mute rounded-sm text-xs"
					style="padding: 3px 7px">⌘K</span
				>
			</div>

			<button
				type="button"
				role="menuitem"
				onclick={() => pick('you')}
				class="border-paper-edge flex w-full cursor-pointer items-center gap-3 border-b text-left {isYou
					? 'bg-accent-soft'
					: 'bg-transparent'}"
				style="padding: 12px"
			>
				<KanjiToken char="携" size="base" toneClass="text-accent" w={20} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm font-medium">Your work</div>
					<div class="text-ink-mute text-xs">every dōjō · nothing to switch</div>
				</div>
				{#if isYou}<span class="text-accent text-sm" aria-hidden="true">✓</span>{/if}
			</button>

			<button
				type="button"
				role="menuitem"
				onclick={goOrgs}
				class="border-paper-edge flex w-full cursor-pointer items-center gap-3 border-b bg-transparent text-left"
				style="padding: 12px"
			>
				<KanjiToken char="全" size="base" toneClass="text-ink-mute" w={20} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm font-medium">All organizations</div>
					<div class="text-ink-mute text-xs">browse every dōjō you can join</div>
				</div>
			</button>

			<div
				class="text-ink-mute text-xs font-semibold uppercase"
				style="letter-spacing: 0.18em; padding: 12px 12px 4px"
			>
				My dōjōs
			</div>
			<div style="max-height: 260px; overflow: auto; padding-bottom: 4px">
				{#each dojos as d (d.slug)}
					{@const on = !isYou && org?.slug === d.slug}
					<button
						type="button"
						role="menuitem"
						onclick={() => pick(d.slug)}
						class="flex w-full cursor-pointer items-center gap-3 border-none text-left {on
							? 'bg-paper-soft'
							: 'bg-transparent'}"
						style="padding: 8px 12px"
					>
						<span
							class="kanji flex-shrink-0 text-center text-sm {kindTone(d.kind).text}"
							style="width: 20px">{d.kanji}</span
						>
						<div class="flex-1" style="min-width: 0">
							<div class="text-ink truncate text-sm">{d.name}</div>
							<div class="mono text-ink-faint text-xs">{K2_ROLE[d.role]?.label} · {d.kind}</div>
						</div>
						{#if (d.needs ?? 0) > 0}
							<span
								class="mono bg-accent text-on-primary rounded-full text-xs font-semibold"
								style="padding: 0 7px; line-height: 16px">{d.needs}</span
							>
						{/if}
						{#if on}<span class="text-accent text-sm" aria-hidden="true">✓</span>{/if}
					</button>
				{/each}
			</div>

			<button
				type="button"
				role="menuitem"
				onclick={goOrgs}
				class="border-paper-edge text-ink-soft flex w-full cursor-pointer items-center gap-2 border-t bg-transparent text-left text-sm"
				style="padding: 12px"
			>
				<span class="text-accent" aria-hidden="true">＋</span> Create or join a dōjō
			</button>
		</div>
	{/if}
</div>
