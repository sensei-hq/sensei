<script lang="ts">
	import { SectionHead, Banner, Btn, Chip, Icon, ListSection } from '$lib/components/kit';
	import type { KitIdentity } from '$lib/components/kit/types';

	// The admin Identity & SSO screen (mockup ScrIdentity) — how org identity
	// connects and how git maps to members. Two config cards (the identity
	// provider's protocol + status + domain, and SCIM provisioning state) over a
	// list of the git / magic-link / device-code mappings roles derive from.
	// Presentational: the page supplies the identity config (kit fixtures this
	// chunk).
	let {
		orgName,
		identity,
		onAddMapping
	}: {
		orgName: string;
		identity: KitIdentity;
		onAddMapping?: () => void;
	} = $props();
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow={orgName + ' · admin'} title="Identity & SSO" />

	<Banner
		kanji="鍵"
		tone="neutral"
		title="Connect org identity — SSO, provisioning, and how git maps to members."
	>
		Roles derive from these mappings; SSO and SCIM keep membership in step with your directory
		automatically.
	</Banner>

	<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
		<!-- Identity provider — protocol + connection status + domain. -->
		<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
			<div class="mb-2 flex items-center gap-2">
				<Icon name="key" size={17} toneClass="text-accent" />
				<span class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.18em"
					>Identity provider</span
				>
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<span class="text-ink text-lg font-medium">{identity.idp.name}</span>
				<Chip mono toneClass="text-ink-mute">{identity.idp.protocol}</Chip>
				<Chip
					icon="check-circle"
					toneClass="text-success"
					softClass="bg-success-soft"
					edgeClass="border-success-soft">{identity.idp.status}</Chip
				>
			</div>
			<div class="mono text-ink-faint text-xs" style="margin-top: 4px">{identity.idp.domain}</div>
			<div style="margin-top: 12px">
				<Btn size="sm" variant="ghost" icon="tuning-2">Configure</Btn>
			</div>
		</div>

		<!-- SCIM provisioning — sync membership from the directory. -->
		<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
			<div class="mb-2 flex items-center gap-2">
				<Icon name="refresh-circle" size={17} toneClass="text-accent" />
				<span class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.18em"
					>SCIM provisioning</span
				>
			</div>
			<div class="text-ink text-sm">
				{identity.scim ? 'Enabled — members sync from your directory' : 'Disabled'}
			</div>
			<div style="margin-top: 12px">
				<Btn size="sm" variant="ghost" icon="tuning-2">Manage</Btn>
			</div>
		</div>
	</div>

	<ListSection icon="link-circle" title="Identity mappings" count={identity.mappings.length}>
		{#snippet right()}
			<Btn size="sm" icon="add-circle" onclick={() => onAddMapping?.()}>Add mapping</Btn>
		{/snippet}
		{#each identity.mappings as m (m.source)}
			<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm">{m.source}</div>
					<div class="mono text-ink-faint text-xs" style="margin-top: 1px">→ {m.to}</div>
				</div>
				<span class="mono text-ink-mute text-xs">{m.count} members</span>
				<Btn size="sm" variant="ghost" icon="pen-2">Edit</Btn>
			</div>
		{/each}
	</ListSection>
</div>
