<script lang="ts">
	import { Card, Button } from '@rokkit/ui';
	import { ScreenShell } from '@rokkit/app';
	import { Notice, Mark } from '@sensei/ui';
	import { dojoLoadNotice } from '$lib/dojo-notices';
	import type { ReadError } from '$lib/read-error';
	import { Chip, ListSection } from '$lib/components/kit';
	import type { KitBilling } from '$lib/components/kit/types';
	import { tierAction, currentTier, attachNotice, SEAT_COPY, hiddenCountLine, hiddenReturnLine } from '$lib/billing-view';
	import type { BillingHiddenOrg } from '$lib/billing-hidden';
	import { NAV_ORG_BASE } from '$lib/nav';
	import { glyphs } from '$lib/glyphs.svelte';
	import { panelsFor, savePanels } from '$lib/view-states.svelte';

	// The admin Plan & billing screen, as the approved mockup draws it
	// (coord/mockups/L4-billing-r3): one notice that says what state the account is
	// in and the next step, the two plans with the current one tagged, why the plan
	// buttons cannot run yet, the seats with who is counted, and the invoices.
	// Free: an organization's public repositories and its members' personal dōjōs.
	// Pro: its private repositories too, billed per seat; the price is to be
	// announced (D46, D52, D59, D75, D86). On rokkit's ScreenShell (D87, view
	// `dojo.org.billing`): search narrows "Who is counted" and opens it; the bands
	// remember how the user left them (view-states). The org's private repositories
	// the plan hides are listed here, for admins only — members are told nothing
	// on Sharing (user ruling 2026-11-05).
	//
	// Presentational: the page supplies the billing state from the live `…/billing`
	// read; the copy and actions come from billing-map and billing-view. A failed
	// read shows the error and claims no plan — never "Free" (D83, WP8).
	let {
		orgName,
		billing,
		error = null,
		hidden = null,
		hiddenError = null
	}: {
		orgName: string;
		billing: KitBilling;
		/** A failed read — shown in place of the plan (#335). */
		error?: ReadError | string | null;
		/** The org's repositories the plan hides (`billing_hidden`), or `null` when none is. */
		hidden?: BillingHiddenOrg | null;
		/** A failed read of them — said in their section, never "none hidden". */
		hiddenError?: ReadError | string | null;
	} = $props();

	const VIEW = 'dojo.org.billing';
	const nav = NAV_ORG_BASE.flatMap((g) => g.items).find((i) => i.id === 'billing');
	let panels = $state(panelsFor(VIEW));
	let filter = $state<{ values: Record<string, unknown>; stat?: string | null }>({ values: {}, stat: null });
	const query = $derived(String(filter.values.q ?? '').trim().toLowerCase());

	const current = $derived(currentTier(billing));
	const notice = $derived(current ? attachNotice(current) : null);
	const offered = $derived(billing.tiers.some((t) => tierAction(t.id, current) !== null));
	const context = $derived(error || !billing.badge ? orgName : `${orgName} · ${billing.badge}`);
	const counted = $derived(
		query
			? billing.seats.filter((s) => s.name.toLowerCase().includes(query) || s.workspaces.toLowerCase().includes(query))
			: billing.seats
	);
	// The first-session info band: what the tab is for, how to read it, the seat count.
	// None over a failed read: there is nothing to read it against.
	const info = $derived(
		error
			? undefined
			: [
					{ label: 'What it’s for', kanji: nav?.kanji ?? '', title: 'The plan decides which of this organization’s repositories the dōjō holds.' },
					{
						label: 'How to read it',
						steps: [
							{ kanji: '記', title: 'The notice says what state the account is in, and what to do.' },
							{ kanji: '札', title: 'Your plan carries its tag; the other card says what it adds.' },
							{ kanji: '群', title: 'Seats count people with a relay session on a private workspace.' }
						]
					},
					{ label: 'Seats', value: billing.seatsActive ?? '—' }
				]
	);
</script>

<ScreenShell
	viewId={VIEW}
	mark={{ kanji: nav?.kanji ?? '', icon: nav?.icon }}
	glyphs={glyphs.mode === 'icon' ? 'icon' : 'text'}
	title="Plan & billing"
	{context}
	description="Public repositories and personal dōjōs are free, and targets that warn are free everywhere. An organization's private repositories sync on Pro, with blocking targets — billed by the seats who work on them."
	search={{ placeholder: 'Search who is counted' }}
	{info}
	bind:filter
	bind:panels
	onpanels={(next) => savePanels(VIEW, next)}
>
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error}
			<Notice size="panel" {...dojoLoadNotice('billing', error)} data-load-error />
		{:else}
			{#if billing.problem}
				<div data-billing-problem><Notice size="panel" {...billing.problem} /></div>
			{/if}

			<!-- The org's private repositories the plan hides, for every user (D88). -->
			{#if hiddenError}
				<section data-billing-hidden aria-labelledby="billing-hidden-h" class="border-line flex flex-col gap-2 rounded border p-4">
					<h2 id="billing-hidden-h" class="text-ink-mute m-0 text-xs font-medium uppercase tracking-widest">Hidden private repositories</h2>
					<Notice size="inline" {...dojoLoadNotice('the hidden repositories', hiddenError)} data-load-error />
				</section>
			{:else if hidden}
				<section data-billing-hidden aria-labelledby="billing-hidden-h" class="border-line flex flex-col gap-2 rounded border p-4">
					<h2 id="billing-hidden-h" class="text-ink-mute m-0 text-xs font-medium uppercase tracking-widest">Hidden private repositories</h2>
					<p class="text-ink m-0 text-sm">{hiddenCountLine(hidden.count)}</p>
					<ul class="m-0 flex flex-col gap-1 pl-4 text-sm">
						{#each hidden.repos as name, i (i)}
							<li class="text-ink">{name}</li>
						{/each}
					</ul>
					<p class="text-ink-mute m-0 text-xs">{hiddenReturnLine(billing)}</p>
				</section>
			{/if}

			{#if billing.showPlans}
				<!-- The two plans; the current one carries its tag. -->
				<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
					{#each billing.tiers as t (t.id)}
						{@const action = tierAction(t.id, current)}
						<Card class={t.current ? 'border-primary' : undefined}>
							<div class="flex h-full flex-col gap-3" data-tier={t.id}>
								<div class="flex items-center gap-2">
									<Mark char={t.kanji} class="text-lg {t.current ? 'text-primary' : 'text-ink-soft'}" />
									<span class="text-ink text-sm font-semibold">{t.name}</span>
									{#if t.current && billing.tag}
										<span data-plan-tag class="bg-primary text-on-primary ml-auto rounded-full px-2 py-0.5 text-xs">{billing.tag}</span>
									{/if}
								</div>
								<div class="flex flex-wrap items-baseline gap-1">
									<span class="display text-ink text-xl font-light">{t.price}</span>
									<span class="text-ink-mute text-xs">{t.sub}</span>
								</div>
								<ul class="m-0 flex list-none flex-col gap-1 p-0">
									{#each t.lines as l (l)}
										<li class="text-ink-mute flex gap-2 text-xs">
											<span class="text-primary flex-shrink-0" aria-hidden="true">·</span>{l}
										</li>
									{/each}
								</ul>
								{#if action}
									<div class="mt-auto">
										<Button
											variant={action.primary ? 'primary' : 'default'}
											style={action.primary ? 'default' : 'outline'}
											disabled
											aria-describedby="attach-note"
											data-action={t.id === 'pro' ? 'attach-pro' : 'move-free'}
											label={action.label}
										/>
									</div>
								{/if}
							</div>
						</Card>
					{/each}
				</div>

				<!-- Why the plan buttons can't run yet (M6); the buttons point here. -->
				{#if offered && notice}
					<div data-attach-notice id="attach-note">
						<Notice size="inline" expanded {...notice} />
					</div>
				{/if}

				<!-- Seats: what is counted today, on its real basis, and who. -->
				<section data-billing-seats aria-labelledby="billing-seats-h" class="border-line flex flex-col gap-3 rounded border p-4">
					<h2 id="billing-seats-h" class="text-ink-mute m-0 text-xs font-medium uppercase tracking-widest">Seats</h2>
					<div class="flex flex-wrap gap-6">
						<div class="flex min-w-0 flex-1 flex-col gap-1">
							<span class="display text-ink text-3xl font-light">{billing.seatsActive ?? '—'}</span>
							<span class="text-ink text-sm">{SEAT_COPY.label}</span>
							<span class="text-ink-mute text-xs">{SEAT_COPY.basis}</span>
							<details data-billing-counted class="mt-2 text-sm" open={query ? true : undefined}>
								<summary class="text-ink-mute cursor-pointer">Who is counted</summary>
								{#if billing.seats.length === 0}
									<p class="text-ink-mute m-0 mt-2">No one has a seat on a private workspace.</p>
								{:else if counted.length === 0}
									<p class="text-ink-mute m-0 mt-2">No one counted matches “{filter.values.q}”.</p>
								{:else}
									<ul class="m-0 mt-2 flex flex-col gap-1 pl-4">
										{#each counted as s, i (i)}
											<li class="text-ink">{s.name} — {s.workspaces}</li>
										{/each}
									</ul>
								{/if}
								<p class="text-ink-mute m-0 mt-2 text-xs">{SEAT_COPY.remedy}</p>
							</details>
						</div>
						<div data-billing-seats-proposed class="border-line flex min-w-0 flex-1 flex-col gap-1 rounded border border-dashed p-3">
							<span class="display text-ink-mute text-3xl font-light">—</span>
							<span class="text-ink text-sm">{SEAT_COPY.proposed.label}</span>
							<span class="text-ink-mute text-xs">{SEAT_COPY.proposed.note}</span>
						</div>
					</div>
				</section>

				<!-- Invoices: none until a payment provider is connected. -->
				<ListSection icon="bill-list" title="Invoices" count={billing.invoices.length}>
					{#if billing.invoices.length === 0}
						<div class="text-ink-soft px-4 py-3 text-sm">No invoices — no payment provider is connected.</div>
					{/if}
					{#each billing.invoices as iv (iv.d)}
						<div class="border-paper-edge flex items-center gap-4 border-b px-4 py-3">
							<span class="mono text-ink-mute flex-1 text-xs">{iv.d}</span>
							<span class="mono text-ink text-sm">{iv.amt}</span>
							<Chip icon="check-circle" toneClass="text-on-success-soft" softClass="bg-success-soft" edgeClass="border-success-soft"
								>{iv.s}</Chip
							>
						</div>
					{/each}
				</ListSection>
			{/if}
		{/if}
	</div>
</ScreenShell>
