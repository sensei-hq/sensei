<script lang="ts">
	import { resolve } from '$app/paths';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import { DojoApiError, exportCompliance, type ExportFormat } from '$lib/client-data';
	import {
		auditPasses,
		bindingsSummary,
		engagementStatusLabel,
		engagementStatusToneClass,
		nonDereferencedCount,
		stripStatusLabel,
		stripStatusToneClass
	} from '$lib/client-view';
	import { clockTime, relativeAge } from '$lib/admin-view';

	// Artifacts audit + compliance export for one engagement (spec: audit trail 録
	// + compliance report). Renders the dereferenced-artifacts audit view
	// (data.artifacts, ALL rows) and the strip status per row. The done-gate is
	// `non_dereferenced == 0`: a non-dereferenced artifact renders a red-fail row
	// AND blocks the compliance export. The export is triggered here; it presents
	// ONLY the source-ref-free covered subset (never a source ref) and surfaces the
	// backend 409 blocked-state (non-dereferenced present) as a blocked banner, not
	// a crash. LEAD floor; a 403 surfaces in the banner.
	let { data } = $props();

	const failing = $derived(nonDereferencedCount(data.artifacts));
	const passes = $derived(auditPasses(data.artifacts));
	const clientLabel = $derived(data.engagement?.client ?? data.engagementId);

	let exportError = $state<string | null>(null);
	let exporting = $state<ExportFormat | null>(null);

	// Trigger the compliance export. On success (csv) offer the file as a download;
	// (json) surface the row count. A 409 blocked-state is surfaced as the export
	// banner — the strip must hold before anything is certified.
	async function runExport(format: ExportFormat) {
		exporting = format;
		exportError = null;
		try {
			const out = await exportCompliance(data.tenantKey, {
				fetch,
				accessToken: data.accessToken,
				engagement: data.engagementId,
				format
			});
			if (out.format === 'csv') {
				download(out.csv, 'text/csv', `compliance-${clientLabel}.csv`);
			} else {
				download(JSON.stringify(out.rows, null, 2), 'application/json', `compliance-${clientLabel}.json`);
			}
		} catch (e) {
			exportError = e instanceof DojoApiError ? e.message : 'could not run the compliance export';
		} finally {
			exporting = null;
		}
	}

	// Client-side download of the exported text (browser only; the button is
	// disabled under the blocked-state so this is only reached on a clean export).
	function download(text: string, type: string, filename: string) {
		if (typeof document === 'undefined') return;
		const url = URL.createObjectURL(new Blob([text], { type }));
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		a.click();
		URL.revokeObjectURL(url);
	}

	const backHref = resolve('/(console)/console/engagements');
</script>

<svelte:head>
	<title>{clientLabel} · audit · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="録"
		eyebrow="Trust · clients · audit"
		title="Artifacts audit"
		sub="Every artifact shared under this engagement, with the universal-strip status. The proof confidentiality held — the export is a compliance report suitable for evidence. It cannot be certified while any artifact is non-dereferenced."
	>
		{#snippet right()}
			<div class="flex items-center gap-2">
				<button
					type="button"
					onclick={() => runExport('csv')}
					disabled={!passes || exporting !== null}
					class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
					style="padding: 8px 13px; cursor: pointer; border: none"
					style:opacity={!passes || exporting !== null ? 0.5 : 1}
					title={passes
						? 'Export the compliance report'
						: 'Blocked: a non-dereferenced artifact is present'}
				>
					<span class="kanji text-xs">録</span>
					{exporting === 'csv' ? 'Exporting…' : 'Export CSV'}
				</button>
				<button
					type="button"
					onclick={() => runExport('json')}
					disabled={!passes || exporting !== null}
					class="border-paper-edge text-ink-soft inline-flex items-center rounded-lg border text-xs font-medium"
					style="padding: 8px 13px; cursor: pointer; background: transparent"
					style:opacity={!passes || exporting !== null ? 0.5 : 1}
				>
					{exporting === 'json' ? 'Exporting…' : 'JSON'}
				</button>
			</div>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 28px">
		<a
			href={backHref}
			class="mono text-ink-mute text-xs no-underline"
			style="display: inline-block; margin-bottom: 14px">← all engagements</a
		>

		{#if data.error}
			<ConsoleBanner>
				Live audit is unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}

		<!-- Engagement summary -->
		<div
			class="bg-paper-soft border-paper-edge flex items-center gap-4 rounded-xl border"
			style="padding: 14px 18px; margin-bottom: 18px"
		>
			<span class="kanji text-accent text-xl">客</span>
			<div class="flex-1" style="min-width: 0">
				<div class="text-ink text-base">{clientLabel}</div>
				{#if data.engagement}
					<div class="mono text-ink-faint truncate text-xs">{bindingsSummary(data.engagement)}</div>
				{/if}
			</div>
			{#if data.engagement}
				<DojoChip toneClass={engagementStatusToneClass(data.engagement.status)}>
					{engagementStatusLabel(data.engagement.status)}
				</DojoChip>
			{/if}
			<span class="mono text-ink-faint text-xs">{data.artifacts.length} artifacts</span>
		</div>

		<!-- The done-gate strip: non_dereferenced == 0 (spec). A red-fail blocks the export. -->
		{#if failing > 0}
			<ConsoleBanner tone="warning" kanji="盾">
				<b class="text-ink font-semibold">{failing}</b> non-dereferenced
				{failing === 1 ? 'artifact' : 'artifacts'} — the strip did not hold. Compliance export is blocked
				until every artifact's source is dropped.
			</ConsoleBanner>
		{:else if data.artifacts.length > 0}
			<ConsoleBanner tone="success" kanji="盾">
				Every artifact is dereferenced — the strip held. This engagement can be certified.
			</ConsoleBanner>
		{/if}

		{#if exportError}
			<ConsoleBanner kanji="録">
				Export blocked. <span class="mono text-ink-mute text-xs">{exportError}</span>
			</ConsoleBanner>
		{/if}

		<div
			class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border"
			style="margin-top: 18px"
		>
			<div
				class="border-paper-edge text-ink-mute grid gap-3 border-b font-semibold uppercase text-xs" style="grid-template-columns: 60px 1fr 90px 1fr 0.8fr; padding: 11px 18px; letter-spacing: 0.1em"
			>
				<span>Seq</span><span>Artifact</span><span>Kind</span><span>Strip status</span><span
					>When</span
				>
			</div>

			{#if data.artifacts.length === 0}
				<div class="text-ink-mute text-sm" style="padding: 20px 18px">
					{data.error
						? 'Could not load the audit view.'
						: 'No artifacts shared under this engagement yet.'}
				</div>
			{:else}
				{#each data.artifacts as a, i (a.id)}
					<div
						class="grid items-center gap-3 {i < data.artifacts.length - 1
							? 'border-paper-edge border-b'
							: ''} {a.dereferenced ? '' : 'bg-danger-soft'}"
						style="grid-template-columns: 60px 1fr 90px 1fr 0.8fr; padding: 12px 18px"
					>
						<span class="mono text-ink-faint text-xs">#{a.seq}</span>
						<span class="text-ink truncate text-sm" title={a.title}>{a.title}</span>
						<span class="mono text-ink-mute text-xs">{a.kind}</span>
						<span>
							<DojoChip toneClass={stripStatusToneClass(a.dereferenced)}>
								{stripStatusLabel(a.dereferenced)}
							</DojoChip>
						</span>
						<span class="mono text-ink-faint text-xs" title={a.created_at}
							>{clockTime(a.published_at ?? a.created_at)} ·
							{relativeAge(a.published_at ?? a.created_at)}</span
						>
					</div>
				{/each}
			{/if}
		</div>

		<div class="text-ink-faint text-xs" style="line-height: 1.55; margin-top: 12px">
			The compliance export carries only the covered, source-ref-free columns — artifact, engagement,
			client, kind, title, strip proof, status and timestamps. It never includes a source reference.
		</div>
	</div>
</div>
