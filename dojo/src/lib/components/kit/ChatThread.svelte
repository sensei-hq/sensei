<script lang="ts">
	import KanjiToken from './KanjiToken.svelte';
	import type { KitChatTurn, KitMe } from './types';

	// A chat thread (kit K2ChatThread) — sensei speaks rarely; the viewer replies.
	// sensei turns lead with the mentor kanji (先) on the left; the viewer's turns
	// mirror to the right and lead with an initials monogram (the kit's
	// window.Avatar, matching the shipped ConsoleTopBar treatment). `me` names the
	// viewer for the byline + monogram.
	let { thread = [], me }: { thread?: KitChatTurn[]; me?: KitMe } = $props();

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

<div class="flex flex-col" style="gap: 16px">
	{#each thread as turn, i (i)}
		{@const mine = turn.who !== 'sensei'}
		<div class="flex gap-3 {mine ? 'flex-row-reverse' : 'flex-row'}">
			{#if mine}
				<span
					class="bg-accent-soft text-accent flex flex-shrink-0 items-center justify-center rounded-full text-xs font-semibold"
					style="width: 28px; height: 28px"
					aria-hidden="true">{initials}</span
				>
			{:else}
				<KanjiToken char={turn.kanji || '先'} size="lg" toneClass="text-accent" />
			{/if}
			<div style="max-width: 460px">
				<div
					class="border-paper-edge rounded-lg border {mine ? 'bg-paper-mute' : 'bg-paper-soft'}"
					style="padding: 12px 16px"
				>
					<div class="text-ink text-sm" style="line-height: 1.5">{turn.text}</div>
				</div>
				<div class="mono text-ink-mute text-xs {mine ? 'text-right' : ''}" style="margin-top: 3px">
					{mine ? (me?.name ?? 'you') : 'sensei'} · {turn.when}
				</div>
			</div>
		</div>
	{/each}
</div>
