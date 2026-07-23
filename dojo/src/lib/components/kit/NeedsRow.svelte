<script lang="ts">
	import Icon from './Icon.svelte';
	import NeedsActions from './NeedsActions.svelte';
	import NeedsResolved from './NeedsResolved.svelte';
	import { needsTone, type NeedsAction } from './needs';
	import type { KitNeed } from './types';

	// One row of the needs-you band (kit K2NeedsRow) — a thing blocked on the
	// viewer. Leads with the kind's icon, then title · project · dōjō · why, and
	// ends in either the per-kind action set or, once acted on, a resolved marker.
	// `resolved` is a map of `{ [need.id]: pastTenseLabel }`; a present key swaps
	// the actions for the marker and dims the row. `stacked` is the phone layout.
	// `onOpen` opens the item; `onAct` forwards the chosen action.
	let {
		item,
		resolved,
		stacked = false,
		onOpen,
		onAct
	}: {
		item: KitNeed;
		resolved?: Record<string, string>;
		stacked?: boolean;
		onOpen?: (item: KitNeed) => void;
		onAct?: (item: KitNeed, action: NeedsAction) => void;
	} = $props();

	const t = $derived(needsTone(item.kind));
	const done = $derived(resolved?.[item.id]);
</script>

{#if stacked}
	<div
		class="border-paper-edge flex w-full flex-col gap-2 border-b"
		style="padding: 12px 16px; opacity: {done ? 0.7 : 1}"
	>
		<button
			type="button"
			onclick={() => onOpen?.(item)}
			class="flex cursor-pointer items-center gap-2 bg-transparent text-left"
		>
			<Icon name={t.icon} size={18} toneClass={t.toneClass} />
			<span class="text-ink flex-1 text-sm font-medium" style="line-height: 1.3">{item.title}</span>
		</button>
		<div class="mono text-ink-mute text-xs">{item.project} · {item.dojo} · {item.why}</div>
		<div class="flex items-center gap-2">
			{#if done}<NeedsResolved label={done} />{:else}<NeedsActions {item} {onAct} />{/if}
			<span class="flex-1"></span>
			<span class="mono text-ink-faint text-xs">{item.age}</span>
		</div>
	</div>
{:else}
	<div
		class="border-paper-edge flex w-full items-center gap-3 border-b"
		style="padding: 12px 16px; opacity: {done ? 0.7 : 1}"
	>
		<button
			type="button"
			onclick={() => onOpen?.(item)}
			class="flex flex-1 cursor-pointer items-center gap-3 bg-transparent text-left"
			style="min-width: 0"
		>
			<span class="flex flex-shrink-0 items-center justify-center" style="width: 22px">
				<Icon name={t.icon} size={19} toneClass={t.toneClass} />
			</span>
			<div class="flex-1" style="min-width: 0">
				<div class="text-ink text-sm font-medium" style="line-height: 1.25">{item.title}</div>
				<div class="mono text-ink-mute text-xs" style="margin-top: 1px">
					{item.project} · {item.dojo} · {item.why}
				</div>
			</div>
		</button>
		{#if done}<NeedsResolved label={done} />{:else}<NeedsActions {item} {onAct} />{/if}
		<span class="mono text-ink-faint flex-shrink-0 text-xs" style="width: 30px; text-align: right"
			>{item.age}</span
		>
	</div>
{/if}
