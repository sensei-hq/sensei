<script lang="ts">
	import { Button } from '@rokkit/ui';
	import { Chip, Icon } from '$lib/components/kit';
	import { personLabel, type ViewerWorkspaceView } from '$lib/viewers-view';
	import { addableMembers, type TeamView } from '$lib/teams-view';
	import type { TeamDraft, SaveTeam } from '$lib/teams-state.svelte';
	import type { KitMember } from '$lib/components/kit/types';

	// A dōjō's teams (#307 A3, D7) — groups of its members for reports by team and
	// for granting a workspace to many people at once. A team grants no access. Each
	// row: the team, its members (each removable), a picker of the dōjō's members
	// not yet on it, a picker granting a workspace to everyone on it, rename and
	// delete. The open draft (New team / Rename) shows a name field. Presentational.
	let {
		teams,
		members,
		workspaces,
		draft,
		onSave,
		onDelete,
		onAddMember,
		onRemoveMember,
		onGrant
	}: {
		teams: TeamView[];
		/** The dōjō's members — who can be put on a team. */
		members: KitMember[];
		/** The dōjō's workspaces — what a team can be granted. */
		workspaces: ViewerWorkspaceView[];
		draft: TeamDraft;
		onSave?: SaveTeam;
		onDelete?: (team: TeamView) => void;
		onAddMember?: (team: TeamView, principalId: string) => void;
		onRemoveMember?: (team: TeamView, principalId: string) => void;
		onGrant?: (team: TeamView, workspaceId: string) => void;
	} = $props();

	/** A picker that fires once per choice, then resets to its prompt. */
	function picked(e: Event & { currentTarget: HTMLSelectElement }, act: (id: string) => void) {
		const id = e.currentTarget.value;
		if (id) act(id);
		e.currentTarget.value = '';
	}
</script>

{#if draft.open}
	<div class="bg-paper-soft border-paper-edge flex flex-wrap items-end gap-3 rounded-lg border p-4">
		<label class="flex min-w-0 flex-1 flex-col gap-1">
			<span class="text-ink-soft text-xs">Team name</span>
			<input
				type="text"
				class="bg-paper border-paper-edge text-ink rounded border px-3 py-2 text-sm"
				value={draft.name}
				oninput={(e) => (draft.name = e.currentTarget.value)}
				onkeydown={(e) => e.key === 'Enter' && onSave && draft.submit(onSave)}
			/>
		</label>
		<Button variant="primary" size="sm" disabled={!onSave} onclick={() => onSave && draft.submit(onSave)}
			>Save team</Button
		>
		<Button style="outline" size="sm" onclick={() => draft.cancel()}>Cancel</Button>
	</div>
{/if}

{#if teams.length === 0}
	<div class="bg-paper-soft border-paper-edge text-ink-soft rounded-lg border p-4 text-sm">
		No teams yet. A team groups members for reports and for granting a workspace to all of them at once.
	</div>
{:else}
	<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
		{#each teams as t (t.id)}
			{@const addable = addableMembers(t, members)}
			<div class="border-paper-edge flex flex-col gap-2 border-b px-4 py-3">
				<div class="flex items-center gap-3">
					<Icon name="users-group-two-rounded" size={18} toneClass="text-ink-soft" />
					<span class="text-ink flex-1 text-sm font-medium">{t.name}</span>
					<span class="text-ink-faint text-xs">{t.members.length} on it</span>
					{#if onSave}
						<button
							type="button"
							class="text-ink-soft cursor-pointer text-xs"
							aria-label={'Rename ' + t.name}
							onclick={() => draft.start(t)}>Rename</button
						>
					{/if}
					{#if onDelete}
						<button
							type="button"
							class="text-ink-soft hover:text-danger cursor-pointer text-xs"
							aria-label={'Delete ' + t.name}
							onclick={() => onDelete?.(t)}>Delete</button
						>
					{/if}
				</div>
				<div class="flex flex-wrap items-center gap-2 pl-8">
					{#each t.members as m (m.principalId)}
						{@const label = personLabel(m)}
						<Chip toneClass="text-ink">
							<span class="inline-flex items-center gap-1">
								{label}
								{#if onRemoveMember}
									<button
										type="button"
										class="cursor-pointer"
										aria-label={'Take ' + label + ' off ' + t.name}
										onclick={() => onRemoveMember?.(t, m.principalId)}
									>
										<Icon name="close-circle" size={13} toneClass="text-ink-faint" />
									</button>
								{/if}
							</span>
						</Chip>
					{:else}
						<span class="text-ink-faint text-xs">No one on this team yet</span>
					{/each}
					{#if onAddMember && addable.length > 0}
						<label class="sr-only" for={'add-' + t.id}>Add a member to {t.name}</label>
						<select
							id={'add-' + t.id}
							class="bg-paper border-paper-edge text-ink rounded border px-2 py-1 text-xs"
							value=""
							onchange={(e) => picked(e, (id) => onAddMember?.(t, id))}
						>
							<option value="">+ member</option>
							{#each addable as m (m.id)}
								<option value={m.id}>{m.name}</option>
							{/each}
						</select>
					{/if}
					{#if onGrant && workspaces.length > 0 && t.members.length > 0}
						<label class="sr-only" for={'grant-team-' + t.id}>Grant a workspace to everyone on {t.name}</label>
						<select
							id={'grant-team-' + t.id}
							class="bg-paper border-paper-edge text-ink rounded border px-2 py-1 text-xs"
							value=""
							onchange={(e) => picked(e, (id) => onGrant?.(t, id))}
						>
							<option value="">grant a workspace…</option>
							{#each workspaces as p (p.id)}
								<option value={p.id}>{p.name}</option>
							{/each}
						</select>
					{/if}
				</div>
			</div>
		{/each}
	</div>
{/if}
