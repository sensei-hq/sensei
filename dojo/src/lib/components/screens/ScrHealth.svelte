<script lang="ts">
	import { SectionHead, Chip, KanjiToken, StatBadge, ListSection } from '$lib/components/kit';
	import type { KitHealth } from '$lib/components/kit/types';
	import { signalToneClass, alertToneClass, barMax, barPct } from '$lib/health-view';

	// The admin Health / Monitor screen (mockup ScrHealth) — the shared mind's
	// vital signs, fed by the audit trail. A grid of signal cards (throughput,
	// adoption, leak-guard, queue age), a contributions-vs-approvals bar chart
	// scaled to the tallest week, and a leak-guard / anomaly alert list.
	// Presentational: the page supplies the health vitals (kit fixtures this
	// chunk); the bar geometry + tones come from the pure `health-view`
	// helpers.
	let {
		orgName,
		health
	}: {
		orgName: string;
		health: KitHealth;
	} = $props();

	const max = $derived(barMax(health.contribVsApprove));
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead
		kanji="観" eyebrow={orgName + ' · admin'} title="Health / Monitor" 
		description="The shared mind's vital signs, fed by the audit trail. Throughput, adoption and leak-guard at a glance. Anomalies surface as alerts before they become incidents."
	/>

	<!-- Signal cards — two-up on phone, four-up on desktop. -->
	<div class="grid grid-cols-2 gap-4 md:grid-cols-4">
		{#each health.signals as s (s.label)}
			<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
				<div class="mb-2 flex items-center gap-2">
					<KanjiToken char={s.kanji} size="base" toneClass={signalToneClass(s.tone)} />
					<span class="text-ink-mute text-xs">{s.label}</span>
				</div>
				<StatBadge n={s.n} label="" sub={s.sub} toneClass={signalToneClass(s.tone)} />
			</div>
		{/each}
	</div>

	<div class="grid grid-cols-1 items-start gap-4 md:grid-cols-[1.3fr_1fr]">
		<!-- Contributions vs approvals — bars scaled to the tallest week. -->
		<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
			<div class="mb-3 flex items-center gap-2">
				<span class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.18em"
					>Contributions vs approvals</span
				>
				<span class="flex-1"></span>
				<Chip toneClass="text-accent" softClass="bg-accent-soft" edgeClass="border-accent-soft"
					>contrib</Chip
				>
				<Chip toneClass="text-success" softClass="bg-success-soft" edgeClass="border-success-soft"
					>approved</Chip
				>
			</div>
			<div class="flex items-end" style="gap: 16px; height: 130px; padding: 0 8px">
				{#each health.contribVsApprove as x (x.wk)}
					<div class="flex flex-1 flex-col items-center gap-2">
						<div class="flex items-end" style="gap: 4px; height: 100px">
							<div
								class="bg-accent"
								style="width: 14px; height: {barPct(x.c, max)}%; border-radius: 3px 3px 0 0"
							></div>
							<div
								class="bg-success"
								style="width: 14px; height: {barPct(x.a, max)}%; border-radius: 3px 3px 0 0"
							></div>
						</div>
						<span class="mono text-ink-faint text-xs">{x.wk}</span>
					</div>
				{/each}
			</div>
		</div>

		<!-- Leak-guard & anomalies — the alert feed. -->
		<ListSection icon="bell" title="Leak-guard & anomalies" count={health.alerts.length}>
			{#each health.alerts as a (a.title)}
				<div class="border-paper-edge flex items-start gap-3 border-b" style="padding: 12px 16px">
					<KanjiToken char={a.kanji} size="base" toneClass={alertToneClass(a.sev)} w={20} />
					<div class="flex-1" style="min-width: 0">
						<div class="text-ink text-sm" style="line-height: 1.3">{a.title}</div>
						<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
							{a.detail} · {a.when}
						</div>
					</div>
				</div>
			{/each}
		</ListSection>
	</div>
</div>
