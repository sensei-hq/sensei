<script lang="ts">
	import { SectionHead, Banner, Chip, KanjiToken, StatBadge, ListSection } from '$lib/components/kit';
	import type { KitBilling } from '$lib/components/kit/types';
	import { monthlyTotal, relayTone, tierCtaLabel } from '$lib/billing-view';

	// The admin Plan & billing screen (mockup ScrBilling) — the business model
	// made legible: free where public or personal, paid where private and shared.
	// A plan summary row (current plan · billable seats · live monthly total), the
	// three tiers side by side (Enterprise on a dark field), the relay
	// free-vs-paid breakdown, and the invoice history. Presentational: the page
	// supplies the billing state (kit fixtures this chunk); the live total + tones
	// come from the pure `billing-view` helpers.
	let {
		orgName,
		billing,
		mobile = false
	}: {
		orgName: string;
		billing: KitBilling;
		mobile?: boolean;
	} = $props();

	const monthly = $derived(monthlyTotal(billing));
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow={orgName + ' · plan & billing'} title="Plan & billing">
		{#snippet right()}
			<Chip toneClass="text-accent" softClass="bg-accent-soft" edgeClass="border-accent-soft"
				>{billing.plan}</Chip
			>
		{/snippet}
	</SectionHead>

	<Banner
		kanji="円"
		tone="neutral"
		title="Free where public or personal; paid where private and shared."
	>
		Seats bill per active contributor. The desktop app, the global collective, bring-your-own-key
		inference, and read-only membership are always free — you pay to coordinate a group, never for
		tokens.
	</Banner>

	<!-- Plan summary — current plan · billable seats · live monthly total. -->
	<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
		<div class="bg-paper-soft border-accent rounded-lg border" style="padding: 16px">
			<div class="text-ink-mute mb-1 text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
				Current plan
			</div>
			<div class="display text-2xl font-light" style="letter-spacing: -0.01em">{billing.plan}</div>
			<div class="text-ink-mute text-sm" style="margin-top: 2px">
				Renews {billing.renews} · <span class="mono">${billing.perSeat}</span>/contributor/mo
			</div>
		</div>
		<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
			<div class="text-ink-mute mb-1 text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
				Billable seats
			</div>
			<StatBadge
				n={billing.seatsActive}
				label="active contributors"
				sub={billing.seatsReadonly + ' read-only free'}
			/>
		</div>
		<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
			<div class="text-ink-mute mb-1 text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
				This month
			</div>
			<StatBadge
				n={'$' + monthly}
				label={billing.seatsActive + ' × $' + billing.perSeat}
				sub="updated live"
				toneClass="text-accent"
			/>
		</div>
	</div>

	<!-- The three tiers — Enterprise on a dark ink field. -->
	<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
		{#each billing.tiers as t (t.id)}
			<div
				class="flex flex-col gap-2 rounded-lg border {t.dark
					? 'bg-ink border-transparent'
					: t.current
						? 'bg-paper-soft border-accent'
						: 'bg-paper-soft border-paper-edge'}"
				style="padding: 16px"
			>
				<div class="flex items-center gap-2">
					<KanjiToken
						char={t.kanji}
						size="lg"
						toneClass={t.dark || t.current ? 'text-accent' : 'text-ink-mute'}
					/>
					<span class="text-sm font-semibold {t.dark ? 'text-on-primary' : 'text-ink'}">{t.name}</span>
					{#if t.current}
						<span class="ml-auto">
							<Chip toneClass="text-accent" softClass="bg-accent-soft" edgeClass="border-accent-soft"
								>current</Chip
							>
						</span>
					{/if}
				</div>
				<div>
					<span class="display text-xl font-light {t.dark ? 'text-on-primary' : 'text-ink'}"
						>{t.price}</span
					>
					<span
						class="mono text-xs {t.dark ? 'text-on-primary opacity-60' : 'text-ink-faint'}"
						style="margin-left: 4px">{t.sub}</span
					>
				</div>
				<div class="flex flex-col gap-1">
					{#each t.lines as l (l)}
						<div
							class="flex gap-2 text-xs {t.dark ? 'text-on-primary opacity-80' : 'text-ink-soft'}"
							style="line-height: 1.45"
						>
							<span class="text-accent flex-shrink-0">·</span>{l}
						</div>
					{/each}
				</div>
				{#if !t.current}
					<button
						type="button"
						class="bg-paper border-paper-edge text-ink hover:bg-paper-mute mt-auto inline-flex cursor-pointer items-center justify-center rounded border text-sm font-medium"
						style="padding: 6px 12px">{tierCtaLabel(t.id)}</button
					>
				{/if}
			</div>
		{/each}
	</div>

	<!-- Relay — free for individuals, paid where it's shared. -->
	<ListSection icon="chat-round-line" title="Relay — free for individuals, paid where it's shared">
		{#each billing.relayRows as r (r.label)}
			{@const tone = relayTone(r.free)}
			<div class="border-paper-edge flex items-center gap-3 border-b" style="padding: 12px 16px">
				<span class="text-ink-soft flex-1 text-sm">{r.label}</span>
				<Chip toneClass={tone.text} softClass={tone.soft} edgeClass={tone.edge}>{tone.label}</Chip>
			</div>
		{/each}
	</ListSection>

	<!-- Invoice history. -->
	<ListSection icon="bill-list" title="Invoices" count={billing.invoices.length}>
		{#each billing.invoices as iv (iv.d)}
			<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
				<span class="mono text-ink-soft flex-1 text-xs">{iv.d}</span>
				<span class="mono text-ink text-sm">{iv.amt}</span>
				<Chip
					icon="check-circle"
					toneClass="text-success"
					softClass="bg-success-soft"
					edgeClass="border-success-soft">{iv.s}</Chip
				>
			</div>
		{/each}
	</ListSection>
</div>
