<script lang="ts">
	// The invite-accept result page (F3b). On success the loader redirects into the
	// joined dōjō, so this renders only when the redemption was rejected — showing
	// the honest reason (wrong email / expired / used / unknown), never a fake join.
	let { data } = $props();
</script>

<svelte:head><title>Accept invite · Dōjō</title></svelte:head>

<div class="flex flex-col items-center justify-center p-8 gap-6" style="min-height: 60vh">
	<div
		class="border-paper-edge bg-paper-soft flex max-w-md flex-col items-center rounded-lg border p-8 text-center gap-3"
	>
		<span class="kanji text-ink-faint text-4xl">封</span>
		<div class="display text-ink text-lg">This invite couldn't be accepted.</div>
		{#if data.error}
			<div class="text-ink-soft text-sm">{data.error}</div>
		{/if}
		<a
			href={data.back}
			class="bg-primary text-on-primary mt-2 rounded-lg px-4 py-2 text-sm font-medium no-underline"
			>Back to your dōjōs</a
		>
	</div>
</div>
