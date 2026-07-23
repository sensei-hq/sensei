<script lang="ts">
	import Icon from './Icon.svelte';
	import { dojoBuild } from '$lib/version';
	import type { KitNavGroup, KitNavItem } from './types';

	// Left nav (kit K2NavPane) — grouped items + a version footer. Mobile-first:
	// an off-canvas drawer under md (a backdrop dims + closes), a static sidebar at
	// md:+ (where `open` is irrelevant), matching the shipped ConsoleNav. Each item
	// renders an i-glyph icon or a kanji glyph + label + optional count badge.
	// Presentational: selecting emits `onnav(id)`; the drawer closes via `onclose`.
	let {
		groups = [],
		active,
		open = false,
		onnav,
		onclose
	}: {
		groups?: KitNavGroup[];
		active?: string;
		open?: boolean;
		onnav?: (id: string) => void;
		onclose?: () => void;
	} = $props();

	function select(it: KitNavItem) {
		onnav?.(it.id);
		onclose?.();
	}
</script>

{#if open}
	<button
		type="button"
		aria-label="Close navigation"
		onclick={() => onclose?.()}
		class="bg-ink fixed inset-0 z-40 cursor-pointer border-none opacity-50 md:hidden"
	></button>
{/if}
<aside
	id="kit-nav"
	class="border-paper-edge bg-paper-soft fixed inset-y-0 left-0 z-50 flex w-[222px] flex-shrink-0 flex-col overflow-auto border-r px-3 py-4 transition-transform duration-200 md:static md:z-auto md:translate-x-0 md:transition-none {open
		? 'translate-x-0'
		: '-translate-x-full'}"
>
	{#each groups as grp, gi (grp.group ?? gi)}
		<div class="mb-4">
			{#if grp.group}
				<div
					class="text-ink-mute text-xs font-semibold uppercase"
					style="letter-spacing: 0.18em; padding: 0 8px; margin-bottom: 8px"
				>
					{grp.group}
				</div>
			{/if}
			<div class="flex flex-col gap-1">
				{#each grp.items as it (it.id)}
					{@const on = active === it.id}
					<button
						type="button"
						onclick={() => select(it)}
						aria-current={on ? 'page' : undefined}
						class="grid w-full items-center gap-2 rounded-lg text-sm {on
							? 'bg-paper border-paper-edge text-ink border'
							: 'text-ink-soft border border-transparent bg-transparent'}"
						style="grid-template-columns: auto 1fr auto; text-align: left; padding: 8px"
					>
						<span class="flex items-center justify-center" style="width: 16px">
							{#if it.icon}
								<Icon name={it.icon} size={17} toneClass={on ? 'text-accent' : 'text-ink-mute'} />
							{:else}
								<span class="kanji text-sm {on ? 'text-accent' : 'text-ink-mute'}">{it.kanji}</span>
							{/if}
						</span>
						<span class="truncate">{it.label}</span>
						{#if it.badge != null}
							<span
								class="mono bg-accent text-on-primary rounded-full text-xs font-semibold"
								style="padding: 0 7px; line-height: 16px">{it.badge}</span
							>
						{:else}
							<span></span>
						{/if}
					</button>
				{/each}
			</div>
		</div>
	{/each}

	<span class="flex-1" style="min-height: 12px"></span>
	<div
		class="mono border-paper-edge text-ink-faint flex items-center gap-1 border-t text-xs"
		style="padding: 12px 8px 0"
		title={dojoBuild.builtAt}
	>
		<span class="kanji">結</span>Dōjō v{dojoBuild.version}
	</div>
</aside>
