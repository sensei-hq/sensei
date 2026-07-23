<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import DojoJoinEmpty from '$lib/components/DojoJoinEmpty.svelte';
	import { kindToneClass, type DojoOrg } from '$lib/dojo-data';
	import {
		clientMembershipCount,
		followsForMembership,
		roleForKind
	} from '$lib/developer-view';

	// My teams (mockup DojoDevTeams): one login, every Dōjō the contributor
	// belongs to, and what each one follows. Driven by the caller's REAL
	// memberships (the layout's `data.memberships`) — this is a personal surface,
	// so there is NO join-gate (DJ1): a solo contributor reaches it and sees the
	// honest empty state ("no memberships yet — join or create a Dōjō from the
	// switcher"). The per-membership role + "follows" line come from the sample
	// overlay in developer-view until `/v1` carries them.
	let { memberships }: { memberships: readonly DojoOrg[] } = $props();

	const clientCount = $derived(clientMembershipCount(memberships));
</script>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="群"
		eyebrow="You · memberships"
		title="Your teams & orgs"
		sub="One login, every Dōjō you belong to. A project routes only to the membership it's bound to — findings never cross into an unrelated hive-mind."
	>
		{#snippet right()}
			<DojoChip toneClass="text-ink-soft">
				{memberships.length}
				{memberships.length === 1 ? 'membership' : 'memberships'}
			</DojoChip>
		{/snippet}
	</ConsoleHead>

	{#if memberships.length === 0}
		<DojoJoinEmpty what="your teams" />
	{:else}
		<div class="flex-1 overflow-auto p-4 md:p-6">
			<div class="grid grid-cols-1 gap-3 md:grid-cols-2">
				{#each memberships as m (m.id)}
					<div
						class="bg-paper-soft border-paper-edge rounded-xl border {kindToneClass[m.kind]}"
						style="border-left-width: 3px; padding: 16px"
					>
						<div class="flex items-center gap-3">
							<span class="kanji text-xl flex-shrink-0" style="line-height: 1">{m.kanji}</span>
							<div class="flex-1" style="min-width: 0">
								<div class="text-ink text-base flex items-center gap-2">
									{m.name}
									{#if m.last}
										<DojoChip toneClass="text-accent">active</DojoChip>
									{/if}
								</div>
								<div
									class="mono text-ink-faint text-xs uppercase"
									style="letter-spacing: 0.06em; margin-top: 4px"
								>
									{m.kind}
								</div>
							</div>
						</div>
						<div
							class="text-sm"
							style="display: grid; grid-template-columns: auto 1fr; gap: 8px 12px; margin-top: 12px"
						>
							<span class="text-ink-faint">Role</span>
							<span class="text-ink">{roleForKind(m.kind)}</span>
							<span class="text-ink-faint">Following</span>
							<span class="text-ink-soft">{followsForMembership(m.id)}</span>
						</div>
					</div>
				{/each}
			</div>

			{#if clientCount > 0}
				<div
					class="bg-paper-soft border-paper-edge flex items-center gap-2 rounded-xl border"
					style="margin-top: 16px; padding: 12px 16px"
				>
					<span class="kanji text-accent text-base">客</span>
					<span class="text-ink-soft text-sm flex-1" style="line-height: 1.5">
						On <b class="text-ink font-semibold">client</b> memberships your contributions are
						automatically anonymized — the lesson travels, the client and repo never do.
					</span>
				</div>
			{/if}
		</div>
	{/if}
</div>
