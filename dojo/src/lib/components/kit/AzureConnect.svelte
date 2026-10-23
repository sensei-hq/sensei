<script lang="ts">
	import { Button } from '@rokkit/ui';
	import Icon from './Icon.svelte';
	import ListSection from './ListSection.svelte';
	import { AZURE_REQUIRED_SCOPES, memberSetLine, tokenLine, writeScopesLine } from '$lib/azure-view';
	import type { AzureConnectState } from '$lib/azure-connect-state.svelte';

	// Connect Azure DevOps through Microsoft Entra (#308 R5b, D36/D42). Connect
	// sends the person to Microsoft (a work or school account; personal Microsoft
	// accounts cannot use Azure DevOps through Entra) — no token ever passes
	// through this screen. Back here, the organizations the connect can see are
	// listed to import, with a warning when the consent carries more than read
	// scopes; on an organization dōjō, the connected organizations follow with
	// their state, Sync and Remove. Presentational: the state holds the acts.
	let { state, intro }: { state: AzureConnectState; intro?: string } = $props();

	const warning = $derived(state.grant ? writeScopesLine(state.grant.writeScopes) : null);
	const where = $derived(
		state.grant?.target === 'own'
			? 'Each becomes its own dōjō, its projects workspaces in it — only for a Project Collection Administrator.'
			: 'Each will connect to this dōjō, its projects workspaces here — only for a Project Collection Administrator.'
	);
</script>

<div class="bg-paper-soft border-paper-edge flex flex-col gap-4 rounded-lg border p-4">
	<div class="flex flex-wrap items-center gap-3">
		<Icon name="link-circle" size={18} toneClass="text-accent" />
		<span class="text-ink min-w-0 flex-1 text-sm font-medium">Azure DevOps</span>
		{#if state.configured !== false}
			<Button variant="primary" size="sm" loading={state.busy === 'connect'} disabled={state.busy !== null} onclick={() => state.connect()}
				>Connect Azure DevOps</Button
			>
		{/if}
	</div>
	{#if intro}<p class="text-ink-mute text-xs">{intro}</p>{/if}

	{#if state.configured === false}
		<div class="text-ink-mute text-sm">Azure DevOps is not configured on this dōjō</div>
	{:else}
		<div class="flex flex-col gap-1">
			<span class="text-ink-mute text-xs"
				>You sign in with a Microsoft work or school account. Personal Microsoft accounts cannot connect Azure DevOps.
				The dōjō asks only to read:</span
			>
			<ul class="text-ink-faint flex flex-col gap-1 pl-4 text-xs">
				{#each AZURE_REQUIRED_SCOPES as s (s.scope)}
					<li>{s.label} — to {s.why}</li>
				{/each}
			</ul>
		</div>
	{/if}

	{#if state.error}
		<div class="bg-danger-soft border-danger-soft text-ink rounded-lg border p-3 text-sm" role="alert">{state.error}</div>
	{/if}
	{#if state.message}
		<div class="text-ink text-sm" role="status">{state.message}</div>
	{/if}

	{#if state.grant}
		{#if warning}
			<div class="bg-warning-soft border-warning-soft text-ink rounded-lg border p-3 text-sm">{warning}</div>
		{/if}
		{#if !state.grant.complete}
			<div class="text-ink-mute text-xs">Azure DevOps could not list every organization you belong to.</div>
		{/if}
		<ListSection icon="buildings-2" title="Organizations to import" count={state.grant.organizations.length}>
			{#each state.grant.organizations as o (o.id)}
				<div class="border-paper-edge flex items-center gap-3 border-b px-4 py-3">
					<span class="text-ink min-w-0 flex-1 text-sm">{o.name}</span>
					<Button
						style="outline"
						size="sm"
						loading={state.busy === o.name}
						disabled={state.busy !== null}
						aria-label={'Import ' + o.name}
						onclick={() => state.importOrg(o.name)}>Import</Button
					>
				</div>
			{:else}
				<div class="text-ink-mute p-4 text-sm">Your Microsoft account belongs to no Azure DevOps organization.</div>
			{/each}
		</ListSection>
		<span class="text-ink-faint text-xs">{where}</span>
	{/if}

	{#if state.connections.length}
		<ListSection icon="buildings-2" title="Connected organizations" count={state.connections.length}>
			{#each state.connections as c (c.id)}
				<div class="border-paper-edge flex flex-col gap-2 border-b px-4 py-3 md:flex-row md:items-center">
					<div class="flex min-w-0 flex-1 flex-col gap-1">
						<span class="text-ink text-sm font-medium">{c.name}</span>
						<span class={c.token === 'ok' ? 'text-ink-mute text-xs' : 'text-warning text-xs'}>{tokenLine(c)}</span>
						<span class={c.memberSetStale ? 'text-warning text-xs' : 'text-ink-mute text-xs'}>{memberSetLine(c)}</span>
					</div>
					<div class="flex flex-wrap items-center gap-2">
						<Button
							style="outline"
							size="sm"
							disabled={state.busy !== null || c.token !== 'ok'}
							aria-label={'Sync ' + c.name}
							onclick={() => state.sync(c)}>Sync</Button
						>
						{#if c.token !== 'none'}
							<Button
								style="outline"
								size="sm"
								disabled={state.busy !== null}
								aria-label={'Remove the connection to ' + c.name}
								onclick={() => state.removeToken(c)}>Remove</Button
							>
						{/if}
					</div>
				</div>
			{/each}
		</ListSection>
	{/if}
</div>
