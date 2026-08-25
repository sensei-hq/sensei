<script lang="ts">
	import type { KitPlan, KitTask } from './types';
	import { phases, stageState } from './plan';
	import { nodeTone } from './vocab';
	import Icon from './Icon.svelte';
	import Chip from './Chip.svelte';

	// The plan outline (kit K2PlanOutline): phases with a paper-mute header
	// (title · roll-up state · done/total), each task a row with its state icon,
	// title, gate chip, agent · model · spec_ref · summary meta, waits-on deps,
	// and the state label. Selectable.
	let {
		plan,
		selectedId,
		onSelect
	}: {
		plan: KitPlan;
		selectedId?: string;
		onSelect?: (task: KitTask) => void;
	} = $props();

	const ps = $derived(phases(plan));

	function meta(t: KitTask): string {
		return [[t.agent, t.model].filter(Boolean).join(' · '), t.spec_ref, t.summary]
			.filter(Boolean)
			.join(' · ');
	}
	function doneCount(tasks: KitTask[]): number {
		return tasks.filter((t) => t.state === 'done' || t.state === 'skipped').length;
	}
</script>

<div class="flex flex-col">
	{#each ps as p, pi (p.id)}
		{@const st = nodeTone(stageState(p))}
		<div class={pi ? 'border-paper-edge border-t' : ''}>
			<div class="bg-paper-mute flex items-center gap-3 px-4 py-3">
				<span class="text-ink min-w-0 flex-1 text-sm font-medium">{p.title}</span>
				<span class="mono text-xs {st.text}">{st.label}</span>
				<span class="mono text-ink-faint text-xs">{doneCount(p.tasks)}/{p.tasks.length}</span>
			</div>
			{#each p.tasks as t (t.id)}
				{@const n = nodeTone(t.state)}
				<button
					type="button"
					onclick={() => onSelect?.(t)}
					class="border-paper-edge flex w-full flex-col gap-2 border-t px-4 py-3 text-left md:flex-row md:items-center md:gap-3 md:pl-8 {selectedId ===
					t.id
						? 'bg-paper-mute'
						: ''}"
				>
					<span class="flex min-w-0 flex-1 items-center gap-3">
						<Icon name={n.icon} size={16} toneClass={n.text} />
						<span class="min-w-0 flex-1">
							<span class="flex flex-wrap items-center gap-2">
								<span
									class="text-sm {t.state === 'pending' || t.state === 'skipped'
										? 'text-ink-mute'
										: 'text-ink'}">{t.title}</span
								>
								{#if t.is_gate}<Chip
										mono
										toneClass="text-accent"
										softClass="bg-accent-soft"
										edgeClass="border-accent-soft"
										>{t.gate_severity === 'advisory' ? 'gate · advisory' : 'gate · blocking'}</Chip
									>{/if}
							</span>
							{#if meta(t)}<span
									class="mono text-ink-faint block overflow-hidden text-ellipsis whitespace-normal text-xs md:whitespace-nowrap"
									style="margin-top: 3px">{meta(t)}</span
								>{/if}
						</span>
					</span>
					<span class="flex shrink-0 items-center gap-3">
						{#if t.deps?.length}<span class="mono text-ink-faint text-xs"
								>waits on {t.deps.join(', ')}</span
							>{/if}
						<span class="mono text-xs md:w-[88px] md:text-right {n.text}">{n.label}</span>
					</span>
				</button>
			{/each}
		</div>
	{/each}
</div>
