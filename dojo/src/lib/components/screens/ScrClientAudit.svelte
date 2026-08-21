<script lang="ts">
	import { SectionHead, Banner, Btn, Chip, Icon, KanjiToken, EmptyState } from '$lib/components/kit';
	import type { KitClientAuditRow } from '$lib/components/kit/types';

	// The lead client-audit ledger (mockup ScrClientAudit) — an append-only,
	// immutable record of exactly what left a client engagement and what was
	// stripped, entry by entry. Distinct from the admin action-audit: this proves
	// confidentiality held. Each row carries a ✓ (held) or a shield (blocked), the
	// client, and the timestamp; filter/export affordances head it and
	// retention/client-read-access chips foot it. Presentational: the page
	// supplies the ledger (kit fixtures this chunk).
	let {
		orgName,
		entries = [],
		mobile = false
	}: {
		orgName: string;
		entries?: KitClientAuditRow[];
		mobile?: boolean;
	} = $props();
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow={orgName + ' · clients'} title="Client audit trail">
		{#snippet right()}
			<div class="flex gap-2">
				<Btn size="sm" variant="ghost" icon="tuning-2">Filter</Btn>
				<Btn size="sm" variant="ghost" icon="download-minimalistic">Export</Btn>
			</div>
		{/snippet}
	</SectionHead>

	<Banner
		kanji="録"
		tone="neutral"
		title="An immutable ledger of exactly what left and what was stripped."
	>
		Distinct from the admin action-audit — this proves confidentiality held for each client, entry
		by entry. Append-only, exportable as CSV or JSON.
	</Banner>

	{#if !entries.length}
		<EmptyState kanji="録" title="The ledger is empty.">
			Every share and every strip is recorded here, entry by entry. Nothing has crossed a client
			boundary yet.
		</EmptyState>
	{:else}
		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
			{#each entries as r (r.t + r.event)}
				<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
					<KanjiToken char={r.kanji} size="base" toneClass={r.ok ? 'text-ink-mute' : 'text-danger'} w={20} />
					<div class="flex-1" style="min-width: 0">
						<div class="text-ink text-sm">{r.event}</div>
						<div class="mono text-ink-faint text-xs" style="margin-top: 1px">{r.detail}</div>
					</div>
					<Chip toneClass="text-ink-mute">{r.client}</Chip>
					{#if r.ok}
						<Icon name="check-circle" size={16} toneClass="text-success" />
					{:else}
						<Icon name="shield-warning" size={16} toneClass="text-danger" />
					{/if}
					<span class="mono text-ink-faint text-xs" style="width: 64px; text-align: right">{r.t}</span>
				</div>
			{/each}
		</div>
	{/if}

	<div class="flex flex-wrap gap-2">
		<Chip icon="lock-keyhole" toneClass="text-ink-mute">Retention · 7 years</Chip>
		<Chip icon="eye" toneClass="text-ink-mute">Client read-access · Globex on</Chip>
	</div>
</div>
