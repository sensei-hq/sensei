<script lang="ts">
	import { SectionHead, Banner, Btn, Chip, Avatar, RoleTag, ListSection } from '$lib/components/kit';
	import type { KitScopeOwner } from '$lib/components/kit/types';

	// The admin scope-ownership screen (mockup ScrScopes) — who owns and triages
	// each scope's queue, grouped Company · Teams · Stacks. Each row is a scope
	// with its queue depth + SLA and its named owner (Avatar + RoleTag), or an
	// "unowned · fallback" chip when no one owns it. The header carries the
	// standing description; a warning Banner appears ONLY when a scope is unowned,
	// so the band means "something needs you" rather than being ambient chrome.
	// Presentational: the page supplies the rows (kit fixtures this chunk).
	let {
		orgName,
		owners = [],
		onAssign
	}: {
		orgName: string;
		owners?: KitScopeOwner[];
		onAssign?: (o: KitScopeOwner) => void;
	} = $props();

	// The authoring groups, in the mockup's fixed order + icon.
	const groups: { name: string; icon: string }[] = [
		{ name: 'Company', icon: 'buildings-2' },
		{ name: 'Teams', icon: 'users-group-rounded' },
		{ name: 'Stacks', icon: 'layers-minimalistic' }
	];

	const unowned = $derived(owners.filter((r) => !r.owner).length);
	// "3 scope has no owner" was the old copy — agree the verb with the count.
	const unownedPhrase = $derived(
		unowned === 1 ? '1 scope has no owner' : `${unowned} scopes have no owner`
	);
	const rowsFor = (group: string) => owners.filter((r) => r.group === group);
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead
		kanji="規"
		eyebrow={orgName + ' · admin'}
		title="Scopes & policies"
		description="Every scope has a named owner who triages its queue. Anything unowned routes to a fallback maintainer so nothing stalls."
	>
		{#snippet right()}
			<Btn size="sm" icon="add-circle">New scope</Btn>
		{/snippet}
	</SectionHead>

	{#if unowned}
		<!-- A real notice, not ambient chrome: only shown when a queue would route
		     to the fallback maintainer. -->
		<Banner tone="warning" title="{unownedPhrase} — its queue routes to a fallback maintainer.">
			Assign an owner to give the scope an SLA. Nothing stalls in the meantime.
		</Banner>
	{/if}

	{#each groups as g (g.name)}
		{@const items = rowsFor(g.name)}
		{#if items.length}
			<ListSection icon={g.icon} title={g.name} count={items.length}>
				{#each items as r (r.scope)}
					<div
						class="border-paper-edge flex items-center gap-4 border-b"
						style="padding: 12px 16px"
					>
						<div class="flex-1" style="min-width: 0">
							<div class="text-ink text-sm font-medium">{r.scope}</div>
							<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
								{r.queue} in queue · SLA {r.sla}
							</div>
						</div>
						{#if r.owner}
							<div class="flex items-center gap-2">
								<Avatar name={r.owner} size={24} />
								<span class="text-ink text-sm">{r.owner}</span>
								{#if r.role}<RoleTag role={r.role} muted />{/if}
							</div>
						{:else}
							<Chip
								icon="danger-triangle"
								toneClass="text-warning"
								softClass="bg-warning-soft"
								edgeClass="border-warning-soft">unowned · fallback</Chip
							>
						{/if}
						<Btn size="sm" variant="ghost" icon="tuning-2" onclick={() => onAssign?.(r)}>
							{r.owner ? 'Reassign' : 'Assign'}
						</Btn>
					</div>
				{/each}
			</ListSection>
		{/if}
	{/each}
</div>
