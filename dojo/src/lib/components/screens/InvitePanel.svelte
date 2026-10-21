<script lang="ts">
	import { Button, Toggle } from '@rokkit/ui';
	import { MEMBER_ROLES, MEMBERSHIP_KINDS } from '$lib/admin-view';
	import type { InviteState, InviteAccess, InviteKind } from '$lib/invite-state.svelte';

	// The admin invite (#307 A3, D8). A member joins at a role; a viewer is not a
	// member — they read only the projects an admin then grants them. Once the
	// dōjō issues the invite, the accept link is shown for the admin to share.
	let { invite }: { invite: InviteState } = $props();

	const ACCESS_OPTS = [
		{ label: 'Member', value: 'member', description: 'Joins the dōjō at a role' },
		{ label: 'Viewer', value: 'viewer', description: 'Reads only the projects you grant' }
	];
	const ROLE_OPTS = MEMBER_ROLES.map((r) => ({ label: r, value: r }));
	const KIND_OPTS = MEMBERSHIP_KINDS.map((k) => ({ label: k, value: k }));

	const set = (patch: Partial<typeof invite.draft>) => (invite.draft = { ...invite.draft, ...patch });
</script>

<div class="bg-paper-soft border-paper-edge flex flex-col gap-4 rounded-lg border p-4">
	{#if invite.issued}
		<div class="flex flex-col gap-2">
			<div class="text-ink text-sm">
				Invite for <span class="font-medium">{invite.issued.email}</span> as a {invite.issued.access}. Share
				this link — it works once, for that email, for 7 days.
			</div>
			<code class="mono bg-paper border-paper-edge text-ink select-all break-all rounded border p-2 text-xs"
				>{invite.issued.link}</code
			>
			<div class="flex gap-2">
				<Button variant="primary" size="sm" onclick={() => invite.start()}>Invite someone else</Button>
				<Button style="outline" size="sm" onclick={() => invite.cancel()}>Done</Button>
			</div>
		</div>
	{:else}
		<label class="flex flex-col gap-1">
			<span class="text-ink-mute text-xs">Email</span>
			<input
				type="email"
				autocomplete="off"
				class="bg-paper border-paper-edge text-ink rounded border px-3 py-2 text-sm"
				value={invite.draft.email}
				oninput={(e) => set({ email: e.currentTarget.value })}
			/>
		</label>
		<div class="flex flex-col gap-1">
			<span class="text-ink-mute text-xs">Access</span>
			<Toggle
				options={ACCESS_OPTS}
				value={invite.draft.access}
				onchange={(v) => set({ access: v as InviteAccess })}
				size="sm"
				label="Access"
			/>
		</div>
		{#if invite.draft.access === 'member'}
			<div class="flex flex-col gap-1">
				<span class="text-ink-mute text-xs">Role</span>
				<Toggle options={ROLE_OPTS} value={invite.draft.role} onchange={(v) => set({ role: v as string })} size="sm" label="Role" />
			</div>
		{/if}
		<div class="flex flex-col gap-1">
			<span class="text-ink-mute text-xs">Relationship</span>
			<Toggle
				options={KIND_OPTS}
				value={invite.draft.kind}
				onchange={(v) => set({ kind: v as InviteKind })}
				size="sm"
				label="Relationship"
			/>
		</div>
		{#if invite.error}
			<div class="text-danger text-sm" role="alert">{invite.error}</div>
		{/if}
		<div class="flex gap-2">
			<Button variant="primary" size="sm" disabled={invite.sending} onclick={() => invite.submit()}>Send invite</Button>
			<Button style="outline" size="sm" onclick={() => invite.cancel()}>Cancel</Button>
		</div>
	{/if}
</div>
