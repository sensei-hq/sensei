<script lang="ts">
	import KanjiToken from '$lib/components/kit/KanjiToken.svelte';
	import { Button } from '@rokkit/ui';
	import type { RelayNotify } from '$lib/relay/relay-notify.svelte';

	// "Notify me" in the relay inbox header. Renders RelayNotify's view and
	// decides nothing: which affordance, its text and whether it is busy are the
	// state's. `tenantKeys` are the dōjōs the inbox spans.
	let { notify, tenantKeys }: { notify: RelayNotify; tenantKeys: string[] } = $props();

	const view = $derived(notify.view);
</script>

{#if view.kind === 'on'}
	<span class="text-ink-soft text-xs" style="white-space: nowrap" data-relay-notify="on">
		<KanjiToken char="鈴" size="inherit" toneClass="" class="text-accent text-xs" />
		{view.text}
	</span>
{:else if view.kind === 'status'}
	<span class="text-ink-faint text-xs" style="white-space: nowrap" data-relay-notify="status">{view.text}</span>
{:else}
	<div class="flex flex-col items-end" style="gap: 4px" data-relay-notify="button">
		<Button variant="primary" size="sm" onclick={() => notify.enable(tenantKeys)} disabled={view.busy}>
			<KanjiToken char="鈴" size="inherit" toneClass="" class="text-xs" />
			{view.buttonLabel}
		</Button>
		{#if view.error}
			<span class="text-danger text-xs" style="white-space: nowrap">{view.error}</span>
		{/if}
	</div>
{/if}
