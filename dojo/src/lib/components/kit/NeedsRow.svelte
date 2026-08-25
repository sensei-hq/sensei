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
	// the actions for the marker and dims the row. One row at every width: it
	// stacks below `md` and becomes a single line from `md` up.
	// `onOpen` opens the item; `onAct` forwards the chosen action.
	let {
		item,
		resolved,
		onOpen,
		onAct
	}: {
		item: KitNeed;
		resolved?: Record<string, string>;
		onOpen?: (item: KitNeed) => void;
		onAct?: (item: KitNeed, action: NeedsAction) => void;
	} = $props();

	const t = $derived(needsTone(item.kind));
	const done = $derived(resolved?.[item.id]);
</script>

<div
	class="border-paper-edge flex w-full flex-col gap-2 border-b px-4 py-3 md:flex-row md:items-center md:gap-3"
	style="opacity: {done ? 0.7 : 1}"
>
	<button
		type="button"
		onclick={() => onOpen?.(item)}
		class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 bg-transparent text-left md:gap-3"
	>
		<span class="flex flex-shrink-0 items-center justify-center md:w-[22px]">
			<Icon name={t.icon} size={19} toneClass={t.toneClass} />
		</span>
		<div class="min-w-0 flex-1">
			<div class="text-ink text-sm font-medium" style="line-height: 1.25">{item.title}</div>
			<div class="mono text-ink-mute text-xs" style="margin-top: 1px">
				{item.project} · {item.dojo} · {item.why}
			</div>
		</div>
	</button>
	<!-- Below md the decision affordance sits on its own line under the title, with
	     the age pushed to the far end; `md:contents` dissolves this wrapper so from
	     md up the actions and age become direct children of the row again. -->
	<div class="flex items-center gap-2 md:contents">
		{#if done}<NeedsResolved label={done} />{:else}<NeedsActions {item} {onAct} />{/if}
		<span class="flex-1 md:hidden"></span>
		<span class="mono text-ink-faint flex-shrink-0 text-right text-xs md:w-[30px]">{item.age}</span>
	</div>
</div>
