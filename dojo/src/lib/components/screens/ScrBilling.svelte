<script lang="ts">
	import { Card, Button } from '@rokkit/ui';
	import { Screen, Notice, Eyebrow } from '@sensei/ui';
	import { SectionHead, Chip, KanjiToken, StatBadge, ListSection, LoadError } from '$lib/components/kit';
	import type { KitBilling } from '$lib/components/kit/types';
	import { monthlyTotal, tierAction, attachNotice } from '$lib/billing-view';

	// The admin Plan & billing screen — two plans (D46). Free: every public
	// repository, and a personal dōjō's private ones. Org: the organization's
	// private repositories too, per active contributor. A summary row (current
	// plan · billable seats · what Org costs at the live seat count), the two plan
	// cards, and the invoice history. Presentational: the page supplies the
	// billing state from the live `…/billing` read; the copy and actions come from
	// billing-view. Plan changes are shown but cannot run until a payment provider
	// is connected (D-BILLING) — the screen says so rather than pretending (#106).
	let {
		orgName,
		billing,
		error = null
	}: {
		orgName: string;
		billing: KitBilling;
		/** A failed read — shown in place of the empty state (#335). */
		error?: string | null;
	} = $props();

	const monthly = $derived(monthlyTotal(billing));
	const current = $derived(billing.tiers.find((t) => t.current)?.id ?? 'free');
	const onFree = $derived(current === 'free');
	const notice = attachNotice();
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				kanji="円" eyebrow={orgName + ' · plan & billing'} title="Plan & billing"
				description="Public repositories and personal dōjōs are free. An organization's private repositories sync on the Org plan, billed per active contributor. Read-only members, the desktop app and bring-your-own-key inference are always free."
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
			<LoadError what="billing" {error} />
		{:else}
			<!-- Summary — current plan · billable seats · what Org costs. -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
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
							label="active contributors"
							sub={billing.seatsReadonly === null ? undefined : billing.seatsReadonly + ' read-only free'}
						/>
					</div>
				</Card>
				<Card>
					<div class="flex flex-col gap-1">
						<Eyebrow>{onFree ? 'On Org, per month' : 'This month'}</Eyebrow>
						{#if monthly === null}
							<StatBadge n="—" label="no seat count yet" />
						{:else}
							<StatBadge
								n={'$' + monthly}
								label={billing.seatsActive + ' × $' + billing.perSeat}
								sub={onFree ? 'an estimate at today\'s seat count' : 'updated live'}
								toneClass="text-accent"
							/>
						{/if}
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
								<KanjiToken char={t.kanji} size="lg" toneClass={t.current ? 'text-accent' : 'text-ink-soft'} />
								<span class="text-ink text-sm font-semibold">{t.name}</span>
								{#if t.current}
									<span class="ml-auto">
										<Chip toneClass="text-on-accent-soft" softClass="bg-accent-soft" edgeClass="border-accent-soft"
											>current</Chip
										>
									</span>
								{/if}
							</div>
							<div class="flex items-baseline gap-1">
								<span class="display text-ink text-xl font-light">{t.price}</span>
								<span class="mono text-ink-faint text-xs">{t.sub}</span>
							</div>
							<ul class="flex flex-col gap-1">
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
										data-action={t.id === 'org' ? 'attach-org' : 'move-free'}
										label={action.label}
									/>
								</div>
							{/if}
						</div>
					</Card>
				{/each}
			</div>

			{#if onFree}
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
