<script lang="ts">
	import { Button } from '@rokkit/ui';
	import Icon from './Icon.svelte';
	import ListSection from './ListSection.svelte';
	import { AZURE_REQUIRED_SCOPES, tokenLine } from '$lib/azure-view';
	import type { AzureConnectState } from '$lib/azure-connect-state.svelte';

	// Connect Azure DevOps (#308 R5b, D35/D36). The organization URL and a personal
	// access token — in a password field the state empties after every request —
	// sent once to the server, which validates the token (it names the organization
	// and proves a Project Collection Administrator) before storing it in Vault.
	// Azure DevOps does not let a token read its own scopes, so the panel names the
	// read-only scopes to give it rather than checking. On an organization dōjō
	// the connected organizations follow, each with its token's state, Sync and
	// Remove token. Presentational: the state holds the fields and the acts.
	let { state, intro }: { state: AzureConnectState; intro?: string } = $props();

	const idFor = (suffix: string) => `azure-${suffix}`;
</script>

<div class="bg-paper-soft border-paper-edge flex flex-col gap-4 rounded-lg border p-4">
	<div class="flex items-center gap-3">
		<Icon name="link-circle" size={18} toneClass="text-accent" />
		<span class="text-ink text-sm font-medium">Connect Azure DevOps</span>
	</div>
	{#if intro}<p class="text-ink-mute text-xs">{intro}</p>{/if}

	<div class="flex flex-col gap-3 md:flex-row md:items-end">
		<label class="flex min-w-0 flex-1 flex-col gap-1" for={idFor('org')}>
			<span class="text-ink-mute text-xs">Organization URL</span>
			<input
				id={idFor('org')}
				type="text"
				inputmode="url"
				placeholder="https://dev.azure.com/your-organization"
				class="bg-paper border-paper-edge text-ink rounded border px-2 py-1 text-sm"
				value={state.organizationUrl}
				oninput={(e) => (state.organizationUrl = e.currentTarget.value)}
			/>
		</label>
		<label class="flex min-w-0 flex-1 flex-col gap-1" for={idFor('pat')}>
			<span class="text-ink-mute text-xs">Personal access token</span>
			<input
				id={idFor('pat')}
				type="password"
				autocomplete="off"
				spellcheck="false"
				class="bg-paper border-paper-edge text-ink rounded border px-2 py-1 text-sm"
				value={state.token}
				oninput={(e) => (state.token = e.currentTarget.value)}
			/>
		</label>
		<Button variant="primary" size="sm" loading={state.busy === 'connect'} disabled={state.busy !== null} onclick={() => state.connect()}
			>Connect Azure DevOps</Button
		>
	</div>

	<div class="flex flex-col gap-1">
		<span class="text-ink-mute text-xs"
			>Use a read-only personal access token, scoped to this one organization, with only these scopes. You must be a Project Collection
			Administrator. The token is checked before it is stored, kept encrypted, and never shown again.</span
		>
		<ul class="text-ink-faint flex flex-col gap-1 pl-4 text-xs">
			{#each AZURE_REQUIRED_SCOPES as s (s.scope)}
				<li>{s.label} — to {s.why}</li>
			{/each}
		</ul>
	</div>

	{#if state.error}
		<div class="bg-danger-soft border-danger-soft text-ink rounded-lg border p-3 text-sm" role="alert">{state.error}</div>
	{/if}
	{#if state.message}
		<div class="text-ink text-sm" role="status">{state.message}</div>
	{/if}

	{#if state.loaded && state.connections.length}
		<ListSection icon="buildings-2" title="Azure DevOps organizations" count={state.connections.length}>
			{#each state.connections as c (c.id)}
				<div class="border-paper-edge flex flex-col gap-2 border-b px-4 py-3 md:flex-row md:items-center">
					<div class="flex min-w-0 flex-1 flex-col gap-1">
						<span class="text-ink text-sm font-medium">{c.name}</span>
						<span class={c.token === 'ok' ? 'text-ink-mute text-xs' : 'text-warning text-xs'}>{tokenLine(c)}</span>
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
								aria-label={'Remove the token for ' + c.name}
								onclick={() => state.removeToken(c)}>Remove token</Button
							>
						{/if}
					</div>
				</div>
			{/each}
		</ListSection>
	{/if}
</div>
