<script lang="ts">
	import { SectionHead, Banner, Btn, Chip, ListSection, KanjiToken, EmptyState } from '$lib/components/kit';
	import type { KitIncident, KitIncidentDetail } from '$lib/components/kit/types';
	import { severityTone, stateToneClass } from '$lib/incidents-view';
	import IncidentDetailPane from './IncidentDetailPane.svelte';

	// The lead confidentiality-incidents list (mockup ScrIncidents) — near-leaks
	// leak-guard held before client source could leave, one row each (client ·
	// when · severity chip · state dot + an open affordance). A retention +
	// client-read-access chip row sits below. Presentational: the page supplies
	// the incidents (kit fixtures this chunk). Degrades to the shared EmptyState
	// when nothing has needed leak-guard attention.
	let {
		orgName,
		incidents = [],
		detail = null,
		mobile = false,
		onOpen,
		onCloseDetail,
		onReport,
		onResolve,
		onDelete
	}: {
		orgName: string;
		incidents?: KitIncident[];
		/** The loaded detail for the "Open" pane, or null when none is open. */
		detail?: KitIncidentDetail | null;
		mobile?: boolean;
		onOpen?: (i: KitIncident) => void;
		/** Dismiss the open detail pane. */
		onCloseDetail?: () => void;
		/** Report (open) a new incident (lead) — prompts for a title. Absent =
		 *  read-only. */
		onReport?: (title: string) => void;
		/** Resolve an incident (lead) — stamps resolved_at. */
		onResolve?: (i: KitIncident) => void;
		/** Delete an incident (lead). */
		onDelete?: (i: KitIncident) => void;
	} = $props();

	// A minimal report flow consistent with the kit: click Report → a one-line
	// title prompt → onReport. No modal component in the kit yet, so a native
	// prompt keeps it lean (browser-verified) rather than over-building a dialog.
	function report() {
		if (!onReport) return;
		const title = typeof window !== 'undefined' ? window.prompt('New incident — title?')?.trim() : '';
		if (title) onReport(title);
	}
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow={orgName + ' · clients'} title="Incidents" count={incidents.length}>
		{#snippet right()}
			<Btn size="sm" icon="add-circle" onclick={report}>Report</Btn>
		{/snippet}
	</SectionHead>

	<Banner kanji="盾" tone="warning" title="Contain a near-leak fast.">
		Leak-guard holds anything that looks like client source before it leaves. Log the containment,
		set retention, and control client read-access here.
	</Banner>

	{#if !incidents.length}
		<EmptyState kanji="盾" title="No incidents on record.">
			Confidentiality containments land here. Nothing has needed leak-guard attention.
		</EmptyState>
	{:else}
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
					{#if onResolve && it.state !== 'resolved'}
						<Btn size="sm" variant="ghost" icon="check-circle" onclick={() => onResolve?.(it)}>
							Resolve
						</Btn>
					{/if}
					{#if onDelete}
						<Btn size="sm" variant="ghost" icon="trash-bin-trash" onclick={() => onDelete?.(it)}>
							Delete
						</Btn>
					{/if}
				</div>
			{/each}
		</ListSection>
	{/if}

	<IncidentDetailPane {detail} onClose={onCloseDetail} />

	<div class="flex flex-wrap gap-2">
		<Chip icon="lock-keyhole" toneClass="text-ink-mute">Retention · 1 year</Chip>
		<Chip icon="eye-closed" toneClass="text-ink-mute">Client read-access · off</Chip>
	</div>
</div>
