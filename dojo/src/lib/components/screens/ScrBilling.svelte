<script lang="ts">
	import { Card, Button } from '@rokkit/ui';
	import { Screen, Notice, Eyebrow, Mark } from '@sensei/ui';
	import { dojoLoadNotice } from '$lib/dojo-notices';
	import type { ReadError } from '$lib/read-error';
	import { SectionHead, Chip, StatBadge, ListSection } from '$lib/components/kit';
	import type { KitBilling } from '$lib/components/kit/types';
	import { tierAction, currentTier, attachNotice } from '$lib/billing-view';

	// The admin Plan & billing screen — two plans (D46, D52, D59, D75). Free: every
	// public repository and a personal dōjō's private ones, with targets. Pro: an
	// organization's private repositories too, with targets, billed by the seats who
	// work on them; its price is to be announced. A summary (current plan, billable
	// seats and what they count), the two plan cards, and the invoice history.
	// Presentational: the page supplies the billing state from the live `…/billing`
	// read; the copy and actions come from billing-view and billing-map. Plan
	// changes are shown but cannot run until a payment provider is connected
	// (D-BILLING), and the screen says so whenever one is offered (#106, M6).
	let {
		orgName,
		billing,
		error = null
	}: {
		orgName: string;
		billing: KitBilling;
		/** A failed read — shown in place of the empty state (#335). */
		error?: ReadError | string | null;
	} = $props();

	const current = $derived(currentTier(billing));
	const notice = $derived(current ? attachNotice(current) : null);
	const offered = $derived(billing.tiers.some((t) => tierAction(t.id, current) !== null));
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				kanji="円" eyebrow={orgName + ' · plan & billing'} title="Plan & billing"
				description="Public repositories and personal dōjōs are free, and targets that warn are free everywhere. An organization's private repositories sync on Pro, with blocking targets — billed by the seats who work on them. SSO, SCIM, audit, viewers and the relay's team queue are free on every plan."
			>
				{#snippet right()}
					{#if billing.plan}
						<Chip toneClass="text-on-accent-soft" softClass="bg-accent-soft" edgeClass="border-accent-soft"
							>{billing.plan}</Chip
						>
					{/if}
				{/snippet}
			</SectionHead>
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if error}
			<Notice size="panel" {...dojoLoadNotice('billing', error)} data-load-error />
		{:else}
			{#if billing.problem}
				<div data-billing-problem><Notice size="panel" {...billing.problem} /></div>
			{/if}

			<!-- Summary — current plan · billable seats and what they count. -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
				<Card class="border-accent">
					<div class="flex flex-col gap-1">
						<Eyebrow>Current plan</Eyebrow>
						<div class="display text-ink text-2xl font-light">{billing.plan ?? '—'}</div>
						{#if billing.renews}
							<div class="text-ink-soft text-sm">Renews {billing.renews}</div>
						{/if}
					</div>
				</Card>
				<Card>
					<div class="flex flex-col gap-1">
						<Eyebrow>Billable seats</Eyebrow>
						<StatBadge
							n={billing.seatsActive ?? '—'}
							label="seats on private workspaces"
							sub={billing.seatsReadonly === null ? undefined : billing.seatsReadonly + ' read-only free'}
						/>
						<p class="text-ink-soft m-0 text-xs">
							People with a seat on a private workspace. Contributors to public repositories and viewers are never counted.
						</p>
					</div>
				</Card>
			</div>

			<!-- The two plans. -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
				{#each billing.tiers as t (t.id)}
					{@const action = tierAction(t.id, current)}
					<Card class={t.current ? 'border-accent' : undefined}>
						<div class="flex h-full flex-col gap-3" data-tier={t.id}>
							<div class="flex items-center gap-2">
								<Mark char={t.kanji} class="text-lg {t.current ? 'text-accent' : 'text-ink-soft'}" />
								<span class="text-ink text-sm font-semibold">{t.name}</span>
								{#if t.current}
									<span class="ml-auto">
										<Chip toneClass="text-on-accent-soft" softClass="bg-accent-soft" edgeClass="border-accent-soft"
											>current</Chip
										>
									</span>
								{/if}
							</div>
							<div class="flex flex-wrap items-baseline gap-1">
								<span class="display text-ink text-xl font-light">{t.price}</span>
								<span class="mono text-ink-faint text-xs">{t.sub}</span>
							</div>
							<ul class="m-0 flex list-none flex-col gap-1 p-0">
								{#each t.lines as l (l)}
									<li class="text-ink-mute flex gap-2 text-xs">
										<span class="text-accent flex-shrink-0" aria-hidden="true">·</span>{l}
									</li>
								{/each}
							</ul>
							{#if action}
								<div class="mt-auto">
									<Button
										variant={action.primary ? 'primary' : 'default'}
										style={action.primary ? 'default' : 'outline'}
										disabled
										data-action={t.id === 'pro' ? 'attach-pro' : 'move-free'}
										label={action.label}
									/>
								</div>
							{/if}
						</div>
					</Card>
				{/each}
			</div>

			{#if offered && notice}
				<div data-attach-notice>
					<Notice size="inline" expanded {...notice} />
				</div>
			{/if}

			<!-- Invoice history. -->
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
	</div>
</Screen>
