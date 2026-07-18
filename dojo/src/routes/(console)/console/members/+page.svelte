<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import { addMember, DojoApiError, setMemberRole, type NewMembershipBody } from '$lib/admin-data';
	import { MEMBER_ROLES, relativeAge, roleLabel, roleToneClass, shortId } from '$lib/admin-view';

	// Members & roles (mockup DojoMembers, dojo-console.jsx l.649). Lists the
	// tenant's memberships (data.members from list_members) and lets an admin set a
	// member's role (PATCH …/members/{user_id}/role) or provision a new membership
	// (POST …/members). Roles are git-derived by default, capped at the least
	// privilege, then fine-tuned here. All mutations are gated by the backend ADMIN
	// floor — a 403 (non-admin) surfaces in the banner rather than crashing.
	let { data } = $props();

	// Per-row + form mutation state (a single in-flight action id + a shared error).
	let actionError = $state<string | null>(null);
	let busyUser = $state<string | null>(null);

	async function changeRole(userId: string, role: string) {
		busyUser = userId;
		actionError = null;
		try {
			await setMemberRole(data.tenantKey, userId, role, {
				fetch,
				accessToken: data.accessToken
			});
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not update the role';
		} finally {
			busyUser = null;
		}
	}

	// Provision form (mockup Invite): the minimum the POST body needs.
	let showInvite = $state(false);
	let formUser = $state('');
	let formVia = $state('sso');
	let formGitRole = $state('');
	let formRole = $state('');
	let inviting = $state(false);

	const canInvite = $derived(formUser.trim().length > 0 && !inviting);

	async function invite() {
		if (!canInvite) return;
		inviting = true;
		actionError = null;
		const body: NewMembershipBody = {
			user_id: formUser.trim(),
			kind: 'human',
			authenticated_via: formVia
		};
		if (formGitRole.trim()) body.git_provider_role = formGitRole.trim();
		if (formRole) body.role = formRole;
		try {
			await addMember(data.tenantKey, body, { fetch, accessToken: data.accessToken });
			formUser = '';
			formGitRole = '';
			formRole = '';
			showInvite = false;
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not provision the member';
		} finally {
			inviting = false;
		}
	}
</script>

<svelte:head>
	<title>Members · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="任"
		eyebrow="Org · members"
		title="Members & roles"
		sub="Roles are derived from git — the highest across a member's associated repos — then fine-tuned here. Auto-provisioning is capped at the least privilege; maintainer and admin are elevated by hand."
	>
		{#snippet right()}
			<button
				type="button"
				onclick={() => (showInvite = !showInvite)}
				class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
				style="padding: 8px 13px; cursor: pointer; border: none"
			>
				<span class="kanji" style="font-size: 12px">任</span>
				{showInvite ? 'Close' : 'Provision member'}
			</button>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 28px">
		{#if data.error}
			<ConsoleBanner>
				Live members are unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}
		{#if actionError}
			<ConsoleBanner>
				Action failed. <span class="mono text-ink-mute text-xs">{actionError}</span>
			</ConsoleBanner>
		{/if}

		<!-- JIT provisioning note (mockup) -->
		<div
			class="bg-paper-soft border-paper-edge border-accent flex items-start gap-3 rounded-xl border"
			style="border-left-width: 3px; padding: 13px 16px; margin: 18px 0"
		>
			<span class="kanji text-accent" style="font-size: 16px; line-height: 1.2">任</span>
			<div class="text-ink-soft text-xs" style="line-height: 1.55">
				<b class="text-ink font-semibold">Just-in-time on first connect.</b> A new member is provisioned
				automatically at their git-derived role — capped at
				<b class="font-semibold">contributor</b>. Maintainer and admin are never granted automatically; elevate
				them by hand below.
			</div>
		</div>

		{#if showInvite}
			<div class="bg-paper-soft border-paper-edge rounded-xl border" style="padding: 16px 18px; margin-bottom: 18px">
				<div class="text-ink-mute font-semibold uppercase" style="font-size: 10px; letter-spacing: 0.1em; margin-bottom: 12px">
					Provision a membership
				</div>
				<div class="grid gap-3" style="grid-template-columns: 1.4fr 1fr 1fr 1fr auto; align-items: end">
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">User id (uuid)</span>
						<input
							class="bg-paper border-paper-edge text-ink rounded-md border text-sm"
							style="padding: 7px 9px"
							bind:value={formUser}
							placeholder="00000000-0000-…"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Authenticated via</span>
						<select class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 7px 9px" bind:value={formVia}>
							<option value="sso">SSO</option>
							<option value="github_oauth">GitHub OAuth</option>
							<option value="device_code">Device code</option>
						</select>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Git role (optional)</span>
						<input
							class="bg-paper border-paper-edge text-ink rounded-md border text-sm"
							style="padding: 7px 9px"
							bind:value={formGitRole}
							placeholder="write"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Role override</span>
						<select class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 7px 9px" bind:value={formRole}>
							<option value="">derive from git</option>
							{#each MEMBER_ROLES as r (r)}
								<option value={r}>{roleLabel(r)}</option>
							{/each}
						</select>
					</label>
					<button
						type="button"
						onclick={invite}
						disabled={!canInvite}
						class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
						style="padding: 8px 13px; cursor: pointer; border: none"
						style:opacity={canInvite ? 1 : 0.5}
					>
						{inviting ? 'Provisioning…' : 'Provision'}
					</button>
				</div>
			</div>
		{/if}

		<div class="bg-paper-soft border-paper-edge overflow-x-auto rounded-xl border">
			<div
				class="border-paper-edge text-ink-mute grid gap-4 border-b font-semibold uppercase min-w-[600px]"
				style="grid-template-columns: 1.6fr 1fr 1.3fr 1fr auto; padding: 11px 18px; font-size: 10px; letter-spacing: 0.1em"
			>
				<span>Member</span><span>Via</span><span>Dōjō role</span><span>Sync</span><span>Active</span>
			</div>

			{#if data.members.length === 0}
				<div class="text-ink-mute text-sm" style="padding: 20px 18px">
					{data.error ? 'Could not load members.' : 'No members yet — provision one above.'}
				</div>
			{:else}
				{#each data.members as m, i (m.id)}
					<div
						class="grid items-center gap-4 min-w-[600px] {i < data.members.length - 1
							? 'border-paper-edge border-b'
							: ''}"
						style="grid-template-columns: 1.6fr 1fr 1.3fr 1fr auto; padding: 13px 18px"
					>
						<div class="flex items-center gap-3" style="min-width: 0">
							<span
								class="bg-accent-soft text-accent flex flex-shrink-0 items-center justify-center rounded-full text-xs font-semibold"
								style="width: 26px; height: 26px"
								aria-hidden="true">{shortId(m.user_id, 2).toUpperCase()}</span
							>
							<span class="mono text-ink truncate text-xs" title={m.user_id}>{shortId(m.user_id, 12)}</span>
						</div>
						<span class="mono text-ink-mute text-xs">{m.authenticated_via}</span>
						<div class="flex items-center gap-2">
							<select
								value={m.role}
								disabled={busyUser === m.user_id}
								onchange={(e) => changeRole(m.user_id, e.currentTarget.value)}
								class="bg-paper border-paper-edge rounded-md border text-sm {roleToneClass(m.role)}"
								style="padding: 4px 8px; cursor: pointer"
								aria-label="Role for {shortId(m.user_id, 8)}"
							>
								{#each MEMBER_ROLES as r (r)}
									<option value={r}>{roleLabel(r)}</option>
								{/each}
							</select>
							{#if busyUser === m.user_id}
								<span class="mono text-ink-faint text-xs">saving…</span>
							{/if}
						</div>
						<span>
							<DojoChip toneClass={m.sync_status === 'synced' ? 'text-success' : 'text-ink-mute'}>
								{m.sync_status}
							</DojoChip>
						</span>
						<span class="mono text-ink-faint text-xs">{relativeAge(m.last_heartbeat_at)}</span>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
