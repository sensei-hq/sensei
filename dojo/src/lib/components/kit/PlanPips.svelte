<script lang="ts">
	import type { KitPlan } from './types';
	import { phases, stageState } from './plan';
	import { nodeTone } from './vocab';

	// Compact per-phase progress strip (kit K2PlanPips): one pip per phase tinted
	// by its roll-up state, a parallel phase splitting into two thin pips. Pip
	// sizes are geometry (inline); colors are token classes. Pending (dashed)
	// phases show a hollow, hairline-bordered pip.
	let { plan, caption = true }: { plan: KitPlan; caption?: boolean } = $props();

	const ps = $derived(phases(plan));
	const total = $derived(ps.reduce((a, p) => a + p.tasks.length, 0));
	const done = $derived(
		ps.reduce((a, p) => a + p.tasks.filter((t) => t.state === 'done').length, 0)
	);

	function pip(p: (typeof ps)[number]) {
		return {
			tone: nodeTone(stageState(p)),
			parallel: p.tasks.filter((t) => !t.deps?.length).length > 1
		};
	}
</script>

{#if ps.length}
	<span class="inline-flex items-center gap-2">
		<span class="flex items-center" style="gap: 3px">
			{#each ps as p (p.id)}
				{@const info = pip(p)}
				<span class="flex" style="gap: 1px" title={p.title}>
					{#each info.parallel ? [0, 1] : [0] as k (k)}
						<span
							class="rounded-full {info.tone.dashed
								? 'border border-paper-edge bg-transparent'
								: info.tone.fill}"
							style="width: {info.parallel ? 5 : 12}px; height: 5px"
						></span>
					{/each}
				</span>
			{/each}
		</span>
		{#if caption}<span class="mono text-ink-faint text-xs">{done}/{total} tasks</span>{/if}
	</span>
{/if}
