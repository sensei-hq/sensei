<script lang="ts">
	import { Avatar, Chip, Icon } from '$lib/components/kit';
	import { personLabel, type DojoViewerView, type ViewerWorkspaceView } from '$lib/viewers-view';

	// The dōjō's viewers (#307 A3, D8) — people invited in as viewers, not members.
	// Each row: who they are, the workspaces granted to them (each with a revoke),
	// a picker of the dōjō's workspaces not yet granted, and remove. Presentational:
	// the page supplies the rows and the writes.
	let {
		viewers,
		workspaces,
		onGrant,
		onRevoke,
		onRemove
	}: {
		viewers: DojoViewerView[];
		/** The dōjō's workspaces — what a viewer can be granted. */
		workspaces: ViewerWorkspaceView[];
		onGrant?: (viewer: DojoViewerView, workspaceId: string) => void;
		onRevoke?: (viewer: DojoViewerView, workspaceId: string) => void;
		onRemove?: (viewer: DojoViewerView) => void;
	} = $props();

	const notYetGranted = (v: DojoViewerView) => workspaces.filter((p) => !v.workspaces.some((g) => g.id === p.id));
</script>

{#if viewers.length === 0}
	<div class="bg-paper-soft border-paper-edge text-ink-soft rounded-lg border p-4 text-sm">
		No viewers yet. Invite a manager or client as a viewer, then grant them the workspaces they should see.
	</div>
{:else}
	<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
		{#each viewers as v (v.principalId)}
			{@const label = personLabel(v)}
			{@const offer = notYetGranted(v)}
			<div class="border-paper-edge flex flex-col gap-2 border-b px-4 py-3">
				<div class="flex items-center gap-4">
					<Avatar name={label} size={30} />
					<div class="min-w-0 flex-1">
						<div class="text-ink text-sm font-medium">{label}</div>
						{#if v.name && v.email}<div class="mono text-ink-faint text-xs">{v.email}</div>{/if}
					</div>
					{#if onRemove}
						<button
							type="button"
							class="text-ink-soft hover:text-danger cursor-pointer text-xs"
							aria-label={'Remove ' + label}
							onclick={() => onRemove?.(v)}>Remove</button
						>
					{/if}
				</div>
				<div class="flex flex-wrap items-center gap-2 pl-12">
					{#each v.workspaces as p (p.id)}
						<Chip toneClass="text-ink">
							<span class="inline-flex items-center gap-1">
								{p.name}
								{#if onRevoke}
									<button
										type="button"
										class="cursor-pointer"
										aria-label={'Revoke ' + p.name + ' from ' + label}
										onclick={() => onRevoke?.(v, p.id)}
									>
										<Icon name="close-circle" size={13} toneClass="text-ink-faint" />
									</button>
								{/if}
							</span>
						</Chip>
					{:else}
						<span class="text-ink-faint text-xs">No workspaces yet</span>
					{/each}
					{#if onGrant && offer.length > 0}
						<label class="sr-only" for={'grant-' + v.principalId}>Grant a workspace to {label}</label>
						<select
							id={'grant-' + v.principalId}
							class="bg-paper border-paper-edge text-ink rounded border px-2 py-1 text-xs"
							value=""
							onchange={(e) => {
								const id = e.currentTarget.value;
								if (id) onGrant?.(v, id);
								e.currentTarget.value = '';
							}}
						>
							<option value="">+ workspace</option>
							{#each offer as p (p.id)}
								<option value={p.id}>{p.name}</option>
							{/each}
						</select>
					{/if}
				</div>
			</div>
		{/each}
	</div>
{/if}
