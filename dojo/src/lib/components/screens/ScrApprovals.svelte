<script lang="ts">
	import { SectionHead, Btn, Chip, KanjiToken, EmptyState } from '$lib/components/kit';
	import type { KitApproval } from '$lib/components/kit/types';
	import { impactTone } from '$lib/triage/candidates-view';

	// The maintainer second-approval queue (mockup ScrApprovals) — high-impact and
	// safety-relevant candidates a first maintainer proposed, each awaiting a
	// second signature before it publishes. One row per approval (first approver ·
	// scope · when + a review/approve affordance). Presentational: the page
	// supplies the approvals (kit fixtures this chunk). Degrades to an honest empty
	// state when the queue is clear (DJ1).
	let {
		orgName,
		approvals = [],
		onReview,
		onApprove
	}: {
		orgName: string;
		approvals?: KitApproval[];
		onReview?: (a: KitApproval) => void;
		onApprove?: (a: KitApproval) => void;
	} = $props();
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead
		kanji="承" eyebrow={orgName + ' · govern'} title="Approvals" count={approvals.length} 
		description="A second maintainer signs off high-impact and safety-relevant candidates. One approval proposes; a second publishes. Nothing safety-relevant reaches a machine on a single signature."
	/>

	{#if approvals.length}
		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
			{#each approvals as a (a.id)}
				{@const im = impactTone(a.impact)}
				<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
					<KanjiToken char={a.kanji} size="lg" toneClass="text-accent" w={22} />
					<div class="flex-1" style="min-width: 0">
						<div class="text-ink text-sm">{a.title}</div>
						<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
							{a.scope} · first approval: {a.first} · {a.when}
						</div>
					</div>
					<Chip toneClass={im.text} softClass={im.soft} edgeClass={im.edge}>{a.impact}</Chip>
					<Btn size="sm" variant="ghost" icon="eye" onclick={() => onReview?.(a)}>Review</Btn>
					<Btn size="sm" icon="check-circle" onclick={() => onApprove?.(a)}>Approve</Btn>
				</div>
			{/each}
		</div>
	{:else}
		<EmptyState kanji="静" title="Nothing awaiting a second look.">
			High-impact candidates land here for a second signature. The queue is clear.
		</EmptyState>
	{/if}
</div>
