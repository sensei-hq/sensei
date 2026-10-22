<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { Button } from '@rokkit/ui';
	import { SectionHead, Chip, Icon, ListSection } from '$lib/components/kit';
	import { dojoLine, mappingLine } from '$lib/integrations-view';
	import type { IntegrationsState } from '$lib/integrations-state.svelte';

	// Admin → Integrations (#308 R5a, D20): a dōjō comes from a git integration.
	// The admin pastes a GitHub token (read:org), reads the organizations it can
	// see, and for each: what its dōjō is (claimed, or who can claim it), where its
	// repositories file, and only the acts this person may take — claim its dōjō,
	// make it its own dōjō, or file it as a workspace of this one. Presentational:
	// the state holds the token, the list and the acts.
	let { orgName, state }: { orgName: string; state: IntegrationsState } = $props();
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				kanji="繋"
				eyebrow={orgName + ' · admin'}
				title="Integrations"
				description="A dōjō comes from a git integration. Read the GitHub organizations your token can see, claim an organization's dōjō if you administer it on GitHub, or file an organization's repositories as a workspace of this dōjō."
			/>
		</div>
	{/snippet}
	<div class="flex flex-col gap-4 p-4 md:gap-6 md:p-8">
		<div class="bg-paper-soft border-paper-edge flex flex-wrap items-end gap-3 rounded-lg border p-4">
			<div class="flex min-w-0 flex-1 flex-col gap-1">
				<label class="text-ink-mute text-xs" for="integrations-token">GitHub token</label>
				<input
					id="integrations-token"
					type="password"
					autocomplete="off"
					class="bg-paper border-paper-edge text-ink rounded border px-3 py-2 text-sm"
					value={state.token}
					oninput={(e) => (state.token = e.currentTarget.value)}
					onkeydown={(e) => e.key === 'Enter' && state.load()}
				/>
				<span class="text-ink-faint text-xs"
					>Needs read:org. Used for these reads only — never stored.</span
				>
			</div>
			<Button variant="primary" size="sm" loading={state.busy === 'list'} onclick={() => state.load()}
				>Read my GitHub organizations</Button
			>
		</div>

		{#if state.error}
			<div class="bg-danger-soft border-danger-soft text-ink rounded-lg border p-4 text-sm" role="alert">
				{state.error}
			</div>
		{/if}

		{#if state.loaded}
			{#if !state.complete}
				<div class="bg-warning-soft border-warning-soft text-ink rounded-lg border p-4 text-sm">
					GitHub could not read every organization this token belongs to; the list below is not complete.
				</div>
			{/if}
			<ListSection icon="link-circle" title="GitHub organizations" count={state.units.length}>
				{#each state.units as u (u.id)}
					<div class="border-paper-edge flex flex-col gap-2 border-b px-4 py-3">
						<div class="flex flex-wrap items-center gap-3">
							<Icon name="buildings-2" size={18} toneClass="text-ink-mute" />
							<span class="text-ink text-sm font-medium">{u.login}</span>
							<Chip toneClass={u.role === 'admin' ? 'text-accent' : 'text-ink-mute'}
								>{u.role === 'admin' ? 'GitHub admin' : 'GitHub member'}</Chip
							>
						</div>
						<div class="flex flex-col gap-1 pl-8">
							<span class="mono text-ink-mute text-xs">{dojoLine(u)}</span>
							<span class="text-ink-faint text-xs">{mappingLine(u)}</span>
						</div>
						{#if u.canClaim || u.canMapAsDojo || u.canMapAsWorkspace}
							<div class="flex flex-wrap items-center gap-2 pl-8">
								{#if u.canClaim}
									<Button
										variant="primary"
										size="sm"
										disabled={state.busy !== null}
										aria-label={'Claim the ' + u.login + ' dōjō'}
										onclick={() => state.claim(u)}>Claim its dōjō</Button
									>
								{/if}
								{#if u.canMapAsDojo}
									<Button
										style="outline"
										size="sm"
										disabled={state.busy !== null}
										aria-label={'Make ' + u.login + ' its own dōjō'}
										onclick={() => state.mapAsDojo(u)}>Its own dōjō</Button
									>
								{/if}
								{#if u.canMapAsWorkspace}
									<label class="sr-only" for={'ws-' + u.id}>Workspace name for {u.login}</label>
									<input
										id={'ws-' + u.id}
										type="text"
										placeholder={u.login}
										class="bg-paper border-paper-edge text-ink rounded border px-2 py-1 text-xs"
										value={state.names[u.id] ?? ''}
										oninput={(e) => (state.names[u.id] = e.currentTarget.value)}
									/>
									<Button
										style="outline"
										size="sm"
										disabled={state.busy !== null}
										aria-label={'File ' + u.login + ' as a workspace of ' + orgName}
										onclick={() => state.mapAsWorkspace(u)}>A workspace here</Button
									>
								{/if}
							</div>
						{/if}
					</div>
				{:else}
					<div class="text-ink-mute p-4 text-sm">This token sees no GitHub organizations.</div>
				{/each}
			</ListSection>
		{/if}
	</div>
</Screen>
