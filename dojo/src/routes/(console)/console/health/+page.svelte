<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import { errorRateToneClass, queueDepthToneClass } from '$lib/admin-view';

	// Health (spec + mockup DojoMonitor Signal cards): the connections / queue-depth
	// / publish-rate / error-rate rollups (data.health from admin_health). Read-only
	// — the four headline signals read straight off dojo.events. Gated by the
	// backend ADMIN floor; a 403 or an unreachable service surfaces in the banner
	// and the cards fall back to `—`.
	let { data } = $props();

	// The four signals, derived from the rollup (null-safe display when unloaded).
	const signals = $derived([
		{
			kanji: '結',
			label: 'Connections',
			value: data.health ? String(data.health.connections) : '—',
			sub: 'distinct members active in the last 5 min',
			tone: 'text-ink'
		},
		{
			kanji: '門',
			label: 'Queue depth',
			value: data.health ? String(data.health.queue_depth) : '—',
			sub: 'candidates queued for triage',
			tone: data.health ? queueDepthToneClass(data.health.queue_depth) : 'text-ink'
		},
		{
			kanji: '決',
			label: 'Publish rate · 1h',
			value: data.health ? String(data.health.publish_rate_1h) : '—',
			sub: 'approved · published · distributed, last hour',
			tone: 'text-success'
		},
		{
			kanji: '警',
			label: 'Error rate · 1h',
			value: data.health ? String(data.health.error_rate_1h) : '—',
			sub: 'error events in the last hour',
			tone: data.health ? errorRateToneClass(data.health.error_rate_1h) : 'text-ink'
		}
	]);
</script>

<svelte:head>
	<title>Health · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="観"
		eyebrow="Trust · monitor"
		title="Hive-mind health"
		sub="Four headline signals read over dojo.events — connections, queue depth, publish rate, and error rate. Keep the shared mind healthy; a backed-up queue or a rising error rate surfaces here first."
	>
		{#snippet right()}
			{#if data.health}
				<span
					class="bg-success-soft border-success-edge text-success mono inline-flex items-center gap-2 rounded-full border text-xs"
					style="padding: 4px 8px"
				>
					● live
				</span>
			{/if}
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 24px">
		{#if data.error}
			<ConsoleBanner>
				Live health is unavailable — showing empty signals.
				<span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}

		<div class="flex gap-4" style="margin-top: 18px">
			{#each signals as s (s.label)}
				<div class="bg-paper-soft border-paper-edge flex-1 rounded-xl border" style="padding: 16px 16px; min-width: 0">
					<div class="flex items-center gap-2" style="margin-bottom: 12px">
						<span class="kanji text-accent text-sm">{s.kanji}</span>
						<span class="text-ink-mute uppercase text-xs" style="letter-spacing: 0.12em">{s.label}</span>
					</div>
					<div class="display font-light {s.tone} text-3xl" style="line-height: 1">{s.value}</div>
					<div class="text-ink-mute text-xs" style="margin-top: 9px; line-height: 1.4">{s.sub}</div>
				</div>
			{/each}
		</div>

		<div
			class="bg-paper-soft border-paper-edge flex items-center gap-2 rounded-xl border"
			style="margin-top: 18px; padding: 12px 16px"
		>
			<span class="kanji text-accent text-sm">録</span>
			<span class="text-ink-soft text-sm" style="line-height: 1.5">
				Every signal resolves to rows in <b class="text-ink font-semibold">dojo.events</b> — the proof a number
				is real. Connections count distinct active members; queue depth is the live triage backlog.
			</span>
		</div>
	</div>
</div>
