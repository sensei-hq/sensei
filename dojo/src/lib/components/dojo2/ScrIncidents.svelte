<script lang="ts">
	import { SectionHead, Banner, Btn, Chip, ListSection, KanjiToken } from '$lib/components/kit';
	import type { KitIncident } from '$lib/components/kit/types';
	import { severityTone, stateToneClass } from '$lib/dojo2-incidents-view';

	// The lead confidentiality-incidents list (mockup ScrIncidents) — near-leaks
	// leak-guard held before client source could leave, one row each (client ·
	// when · severity chip · state dot + an open affordance). A retention +
	// client-read-access chip row sits below. Presentational: the page supplies
	// the incidents (kit fixtures this chunk).
	let {
		orgName,
		incidents = [],
		mobile = false,
		onOpen
	}: {
		orgName: string;
		incidents?: KitIncident[];
		mobile?: boolean;
		onOpen?: (i: KitIncident) => void;
	} = $props();
</script>

<div class="flex flex-col {mobile ? 'p-4 gap-4' : 'p-8 gap-6'}">
	<SectionHead eyebrow={orgName + ' · clients'} title="Incidents" count={incidents.length}>
		{#snippet right()}
			<Btn size="sm" icon="add-circle">Report</Btn>
		{/snippet}
	</SectionHead>

	<Banner kanji="盾" tone="warning" title="Contain a near-leak fast.">
		Leak-guard holds anything that looks like client source before it leaves. Log the containment,
		set retention, and control client read-access here.
	</Banner>

	<ListSection icon="shield-warning" title="Confidentiality incidents" count={incidents.length}>
		{#each incidents as it (it.id)}
			{@const sv = severityTone(it.severity)}
			{@const st = stateToneClass(it.state)}
			<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
				<KanjiToken char={it.kanji} size="lg" toneClass={sv.text} w={22} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm">{it.title}</div>
					<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
						{it.client} · {it.when}
					</div>
				</div>
				<Chip toneClass={sv.text} softClass={sv.soft} edgeClass={sv.edge}>{it.severity}</Chip>
				<span class="inline-flex items-center gap-1 text-xs {st}">
					<span class="rounded-full" style="width: 6px; height: 6px; background: currentColor"></span
					>{it.state}
				</span>
				<Btn size="sm" variant="ghost" icon="alt-arrow-right" onclick={() => onOpen?.(it)}>Open</Btn>
			</div>
		{/each}
	</ListSection>

	<div class="flex flex-wrap gap-2">
		<Chip icon="lock-keyhole" toneClass="text-ink-mute">Retention · 1 year</Chip>
		<Chip icon="eye-closed" toneClass="text-ink-mute">Client read-access · off</Chip>
	</div>
</div>
