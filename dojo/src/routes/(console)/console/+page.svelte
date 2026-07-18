<script lang="ts">
	import { resolve } from '$app/paths';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import Confidence from '$lib/components/Confidence.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import Spark from '$lib/components/Spark.svelte';
	import { metrics } from '$lib/dojo-data';
	import { kindKanji, kindLabel, scopeLabel } from '$lib/triage-view';

	// Overview — the maintainer dashboard (mockup DojoOverview). The pending-triage
	// count + top-of-queue preview are live (data.queue from list_triage); the
	// remaining metric cards, confidentiality note, recent activity and the
	// published-adoption table are the design's static demo content (metrics from
	// $lib/dojo-data) until those endpoints land in a later chunk.
	let { data } = $props();

	const m = metrics;
	const pending = $derived(data.queue.length);
	const preview = $derived(data.queue.slice(0, 4));

	// Static publish-and-measure demo rows (the Impact loop, scoped to the org).
	const published = [
		{ kanji: '守', title: 'Never log refresh tokens', scope: 'Company', adoption: 0.92, delta: 6, neg: false },
		{
			kanji: '紋',
			title: 'Idempotency key on money-moving mutations',
			scope: 'Team · Payments',
			adoption: 0.78,
			delta: 9,
			neg: false
		},
		{
			kanji: '問',
			title: 'Prefer optimistic UI for list mutations',
			scope: 'Stack · React',
			adoption: 0.41,
			delta: -3,
			neg: true
		}
	];

	const activity = [
		{ kanji: '決', tone: 'text-accent', text: 'Keiko approved "Never log refresh tokens" → Company', when: '8m' },
		{ kanji: '共', tone: 'text-ink-mute', text: 'Aiko shared a persona from web-auth', when: '1h' },
		{ kanji: '盾', tone: 'text-success', text: '1 client lesson auto-dereferenced from Globex', when: '2h' },
		{ kanji: '配', tone: 'text-ink-mute', text: '"Idempotency key" distributed to Payments (6 repos)', when: '4h' },
		{ kanji: '改', tone: 'text-ink-mute', text: 'Sven revised "API compatibility" before publishing', when: '1d' }
	];
</script>

<svelte:head>
	<title>Overview · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="全"
		eyebrow="{data.org?.name ?? 'Your org'} · Dōjō"
		title="The shared mind, governed."
		sub="What your org has learned — triaged, approved, and routed to the scopes that need it."
	>
		{#snippet right()}
			<div class="mono text-ink-mute text-right text-xs" style="line-height: 1.7">
				<div>{m.approvedWeek + pending} artifacts · {data.org?.members ?? 0} members</div>
				<div class="text-success">0 confidentiality incidents</div>
			</div>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 28px">
		{#if data.error}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-bottom: 18px"
			>
				<span class="kanji text-warning">検</span>
				<span
					>Live triage is unavailable — showing an empty queue. <span class="mono text-ink-mute text-xs"
						>{data.error}</span
					></span
				>
			</div>
		{/if}

		<!-- metric row -->
		<div class="flex gap-4">
			<a
				href={resolve('/(console)/console/triage')}
				class="bg-paper-soft border-paper-edge flex-1 rounded-xl border no-underline"
				style="padding: 16px 18px; min-width: 0"
			>
				<div class="flex items-center gap-2" style="margin-bottom: 10px">
					<span class="kanji text-accent text-sm">門</span>
					<span class="text-ink-mute uppercase text-xs" style="letter-spacing: 0.12em">Pending triage</span>
				</div>
				<div class="flex items-end justify-between gap-2">
					<div class="display text-ink font-light text-2xl" style="line-height: 1">{pending}</div>
					<span class="mono text-accent text-xs">review →</span>
				</div>
				<div class="text-ink-mute text-xs" style="margin-top: 8px">across your scopes</div>
			</a>

			<div class="bg-paper-soft border-paper-edge flex-1 rounded-xl border" style="padding: 16px 18px; min-width: 0">
				<div class="flex items-center gap-2" style="margin-bottom: 10px">
					<span class="kanji text-accent text-sm">共</span>
					<span class="text-ink-mute uppercase text-xs" style="letter-spacing: 0.12em">Contributions · 7d</span>
				</div>
				<div class="flex items-end justify-between gap-2">
					<div class="display text-ink font-light text-2xl" style="line-height: 1">{m.contribWeek}</div>
					<span class="text-accent"><Spark data={m.contribSpark} /></span>
				</div>
			</div>

			<div class="bg-paper-soft border-paper-edge flex-1 rounded-xl border" style="padding: 16px 18px; min-width: 0">
				<div class="flex items-center gap-2" style="margin-bottom: 10px">
					<span class="kanji text-accent text-sm">決</span>
					<span class="text-ink-mute uppercase text-xs" style="letter-spacing: 0.12em">Approved · 7d</span>
				</div>
				<div class="flex items-end justify-between gap-2">
					<div class="display text-ink font-light text-2xl" style="line-height: 1">{m.approvedWeek}</div>
				</div>
				<div class="text-ink-mute text-xs" style="margin-top: 8px">published to matching scopes</div>
			</div>

			<div class="bg-paper-soft border-paper-edge flex-1 rounded-xl border" style="padding: 16px 18px; min-width: 0">
				<div class="flex items-center gap-2" style="margin-bottom: 10px">
					<span class="kanji text-accent text-sm">果</span>
					<span class="text-ink-mute uppercase text-xs" style="letter-spacing: 0.12em">Adoption lift</span>
				</div>
				<div class="flex items-end justify-between gap-2">
					<div class="display text-ink font-light text-2xl" style="line-height: 1">
						+{Math.round(m.adoptionLift * 100)}pp
					</div>
					<span class="text-success"><Spark data={[6, 7, 6, 9, 8, 10, 11]} /></span>
				</div>
				<div class="text-ink-mute text-xs" style="margin-top: 8px">FTR across adopting scopes</div>
			</div>
		</div>

		<!-- two columns -->
		<div class="grid gap-5" style="grid-template-columns: 1.4fr 1fr; margin-top: 18px">
			<!-- triage preview (live) -->
			<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
				<div class="border-paper-edge flex items-center border-b" style="padding: 13px 16px">
					<span class="text-ink-mute uppercase text-xs" style="letter-spacing: 0.14em">Top of the triage queue</span>
					<span class="flex-1"></span>
					<a
					href={resolve('/(console)/console/triage')}
					class="mono text-accent text-xs no-underline">all {pending} →</a
				>
				</div>
				{#if preview.length === 0}
					<div class="text-ink-mute text-sm" style="padding: 20px 16px">
						Nothing waiting — the queue is clear.
					</div>
				{:else}
					{#each preview as c, i (c.signature)}
						<a
							href={resolve('/(console)/console/triage/[signature]', {
								signature: c.signature
							})}
							class="grid items-center gap-3 no-underline {i < preview.length - 1
								? 'border-paper-edge border-b'
								: ''}"
							style="grid-template-columns: auto 1fr auto; padding: 12px 16px"
						>
							<span class="kanji text-accent text-center text-lg" style="width: 20px">{kindKanji(c.kind)}</span>
							<div style="min-width: 0">
								<div class="text-ink truncate text-sm">{c.title}</div>
								<div class="flex items-center gap-2" style="margin-top: 4px">
									<span class="mono text-ink-mute text-xs">{scopeLabel(c.owner_scope)}</span>
									<DojoChip>{kindLabel(c.kind)}</DojoChip>
								</div>
							</div>
							<Confidence value={c.confidence} width={56} />
						</a>
					{/each}
				{/if}
			</div>

			<!-- confidentiality + recent activity (static demo content) -->
			<div class="flex flex-col gap-5">
				<div class="bg-paper-soft border-success-edge rounded-xl border" style="padding: 15px 17px">
					<div class="flex items-center gap-2" style="margin-bottom: 8px">
						<span class="kanji text-success text-sm">盾</span>
						<span class="text-ink-mute uppercase text-xs" style="letter-spacing: 0.14em">Confidentiality</span>
					</div>
					<div class="text-ink text-sm" style="line-height: 1.55">
						<b class="font-semibold">{m.dereferenced}</b> client lessons auto-dereferenced this week ·
						<span class="text-success">0 incidents</span>. Sources dropped automatically; only flagged exceptions
						reach a lead.
					</div>
					<div
						class="border-paper-edge flex items-center gap-2 border-t"
						style="margin-top: 10px; padding-top: 10px"
					>
						<span class="bg-success rounded-full" style="width: 6px; height: 6px"></span>
						<span class="mono text-ink-mute text-xs uppercase" style="letter-spacing: 0.06em">Leak-guard armed</span>
					</div>
				</div>

				<div class="bg-paper-soft border-paper-edge flex-1 overflow-hidden rounded-xl border">
					<div
						class="border-paper-edge text-ink-mute border-b uppercase text-xs" style="padding: 13px 16px; letter-spacing: 0.14em"
					>
						Recent activity
					</div>
					<div style="padding: 4px 0">
						{#each activity as a (a.text)}
							<div class="grid items-start gap-3" style="grid-template-columns: auto 1fr auto; padding: 9px 16px">
								<span class="kanji text-center {a.tone} text-sm" style="width: 16px">{a.kanji}</span>
								<span class="text-ink-soft text-xs" style="line-height: 1.45">{a.text}</span>
								<span class="mono text-ink-faint text-xs">{a.when}</span>
							</div>
						{/each}
					</div>
				</div>
			</div>
		</div>

		<!-- Published · adoption & health (static demo content) -->
		<div style="margin-top: 18px">
			<div class="flex items-center" style="margin-bottom: 9px">
				<span class="text-ink-mute font-semibold uppercase text-xs" style="letter-spacing: 0.14em"
					>Published · adoption &amp; health</span
				>
				<span class="mono text-ink-faint text-xs" style="margin-left: 10px">the Impact loop, scoped to the org</span>
			</div>
			<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
				{#each published as p, i (p.title)}
					<div
						class="grid items-center gap-4 {i < published.length - 1 ? 'border-paper-edge border-b' : ''} {p.neg
							? 'bg-warning-soft'
							: ''}"
						style="grid-template-columns: auto 1fr 150px 92px 120px; padding: 13px 16px"
					>
						<span class="kanji text-center {p.neg ? 'text-warning' : 'text-accent'} text-lg" style="width: 20px"
							>{p.kanji}</span
						>
						<div style="min-width: 0">
							<div class="text-ink truncate text-sm">{p.title}</div>
							<div class="mono text-ink-faint text-xs" style="margin-top: 3px">{p.scope}</div>
						</div>
						<div class="flex items-center gap-2">
							<div class="bg-paper-mute h-1 flex-1 overflow-hidden rounded-full">
								<div class="bg-accent h-full rounded-full" style="width: {Math.round(p.adoption * 100)}%"></div>
							</div>
							<span class="mono text-ink-mute text-xs">{Math.round(p.adoption * 100)}%</span>
						</div>
						<span class="mono text-right text-xs {p.neg ? 'text-warning' : 'text-success'}">
							{p.neg ? '' : '+'}{p.delta}pp FTR
						</span>
						<div class="flex justify-end">
							{#if p.neg}
								<button
									type="button"
									class="border-warning-edge bg-paper text-warning inline-flex items-center gap-2 rounded-lg border text-xs"
									style="padding: 6px 12px; cursor: pointer"
								>
									<span class="kanji text-xs">退</span> Retract
								</button>
							{:else}
								<DojoChip toneClass="text-success">active</DojoChip>
							{/if}
						</div>
					</div>
				{/each}
				<div
					class="border-paper-edge text-ink-mute flex items-center gap-2 border-t text-xs"
					style="padding: 10px 16px; line-height: 1.45"
				>
					<span class="kanji text-warning text-xs">退</span>
					<span
						>Lifecycle <b class="text-ink-soft font-semibold">active → deprecated → retracted</b>. Negative impact is
						flagged automatically; one-click retract pulls a teaching back and notifies adopters.</span
					>
				</div>
			</div>
		</div>
	</div>
</div>
