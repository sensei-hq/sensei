<script lang="ts">
	import { Button } from '@rokkit/ui';

	// Relay P4.5 — the calm connectivity banner for a run's detail. Purely
	// presentational: it reads props and renders, never touching the store, fetch,
	// or connectivity itself (the page owns all of that). Three quiet states:
	//   • offline           → a soft warning that guidance is being held locally.
	//   • online + queued>0  → an offer to send what's waiting.
	//   • a partial-failure note → surfaced inline so a stuck flush isn't silent.
	// Steady online with nothing queued renders NOTHING — no chrome when all is well.
	let {
		online,
		queued,
		flushing = false,
		onSend,
		note = null
	}: {
		online: boolean;
		queued: number;
		flushing?: boolean;
		onSend?: () => void;
		note?: string | null;
	} = $props();

	// Nothing to say when we're online, the queue is empty, and there's no lingering
	// partial-failure note — the banner stays out of the way.
	const hidden = $derived(online && queued === 0 && !note);
</script>

{#if !hidden}
	{#if !online}
		<!-- Offline — a soft warning; guidance is held locally until reconnect. -->
		<div
			class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
			style="padding: 12px 16px; margin-top: 16px"
		>
			<span class="kanji text-warning">検</span>
			<span>
				You're offline — sensei is holding your guidance here.
				{#if queued > 0}
					{queued} change{queued === 1 ? '' : 's'} queued — they'll send when you're back online.
				{/if}
			</span>
			{#if note}
				<span class="mono text-ink-mute text-xs">{note}</span>
			{/if}
		</div>
	{:else}
		<!-- Online with work waiting — offer to send it. -->
		<div
			class="bg-warning-soft border-warning-edge text-ink-soft flex flex-wrap items-center gap-2 rounded-xl border text-sm"
			style="padding: 12px 16px; margin-top: 16px"
		>
			<span class="kanji text-warning">検</span>
			<span style="flex: 1; min-width: 0">
				You're back online.
				{#if queued > 0}
					{queued} change{queued === 1 ? '' : 's'} still queued.
				{/if}
				{#if note}
					<span class="mono text-ink-mute text-xs">{note}</span>
				{/if}
			</span>
			{#if queued > 0}
				<Button variant="primary" size="sm" onclick={onSend} disabled={flushing}>
					<span class="kanji text-xs">送</span>
					{flushing ? 'Sending…' : 'Send queued'}
				</Button>
			{/if}
		</div>
	{/if}
{/if}
