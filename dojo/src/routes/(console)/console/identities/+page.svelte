<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import {
		createIdentity,
		deleteIdentity,
		DojoApiError,
		type NewIdentityBody
	} from '$lib/admin-data';
	import { AUTH_METHODS, providerKanji, providerLabel, relativeAge } from '$lib/admin-view';

	// Identities (spec: SSO / GitHub / device-code identity config CRUD). Lists the
	// tenant's identity mappings (data.identities from list_identities) and lets an
	// admin wire a new provider (POST …/identities) or remove one
	// (DELETE …/identities/{id}). Gated by the backend ADMIN floor — a 403 surfaces
	// in the banner.
	let { data } = $props();

	let actionError = $state<string | null>(null);
	let busyId = $state<string | null>(null);

	async function remove(id: string) {
		busyId = id;
		actionError = null;
		try {
			await deleteIdentity(data.tenantKey, id, { fetch, accessToken: data.accessToken });
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not remove the identity';
		} finally {
			busyId = null;
		}
	}

	let showAdd = $state(false);
	let formUser = $state('');
	let formProvider = $state('sso');
	let formSubject = $state('');
	let formEmail = $state('');
	let formName = $state('');
	let adding = $state(false);

	const canAdd = $derived(
		formUser.trim().length > 0 && formSubject.trim().length > 0 && !adding
	);

	async function add() {
		if (!canAdd) return;
		adding = true;
		actionError = null;
		const body: NewIdentityBody = {
			user_id: formUser.trim(),
			provider: formProvider,
			subject: formSubject.trim()
		};
		if (formEmail.trim()) body.email = formEmail.trim();
		if (formName.trim()) body.display_name = formName.trim();
		try {
			await createIdentity(data.tenantKey, body, { fetch, accessToken: data.accessToken });
			formUser = '';
			formSubject = '';
			formEmail = '';
			formName = '';
			showAdd = false;
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not wire the identity';
		} finally {
			adding = false;
		}
	}
</script>

<svelte:head>
	<title>Identities · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="鍵"
		eyebrow="Connect · identity"
		title="Identities"
		sub="Wire SSO, GitHub OAuth, and device-code enrolments — no anonymous access. First login for a new member auto-creates their membership at the least privilege."
	>
		{#snippet right()}
			<button
				type="button"
				onclick={() => (showAdd = !showAdd)}
				class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
				style="padding: 8px 13px; cursor: pointer; border: none"
			>
				<span class="kanji" style="font-size: 12px">鍵</span>
				{showAdd ? 'Close' : 'Wire identity'}
			</button>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 28px">
		{#if data.error}
			<ConsoleBanner>
				Live identities are unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}
		{#if actionError}
			<ConsoleBanner>
				Action failed. <span class="mono text-ink-mute text-xs">{actionError}</span>
			</ConsoleBanner>
		{/if}

		{#if showAdd}
			<div class="bg-paper-soft border-paper-edge rounded-xl border" style="padding: 16px 18px; margin: 18px 0">
				<div class="text-ink-mute font-semibold uppercase" style="font-size: 10px; letter-spacing: 0.1em; margin-bottom: 12px">
					Wire an identity provider
				</div>
				<div class="grid gap-3" style="grid-template-columns: 1fr 1fr 1.3fr 1.3fr 1fr auto; align-items: end">
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">User id (uuid)</span>
						<input class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 7px 9px" bind:value={formUser} placeholder="00000000-…" />
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Provider</span>
						<select class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 7px 9px" bind:value={formProvider}>
							{#each AUTH_METHODS as p (p)}
								<option value={p}>{providerLabel(p)}</option>
							{/each}
						</select>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Subject</span>
						<input class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 7px 9px" bind:value={formSubject} placeholder="okta|abc123" />
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Email (optional)</span>
						<input class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 7px 9px" bind:value={formEmail} placeholder="kei@acme.co" />
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Name (optional)</span>
						<input class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 7px 9px" bind:value={formName} placeholder="Keiko" />
					</label>
					<button
						type="button"
						onclick={add}
						disabled={!canAdd}
						class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
						style="padding: 8px 13px; cursor: pointer; border: none"
						style:opacity={canAdd ? 1 : 0.5}
					>
						{adding ? 'Wiring…' : 'Wire'}
					</button>
				</div>
			</div>
		{/if}

		<div class="bg-paper-soft border-paper-edge overflow-x-auto rounded-xl border" style="margin-top: 18px">
			<div
				class="border-paper-edge text-ink-mute grid gap-4 border-b font-semibold uppercase min-w-[600px]"
				style="grid-template-columns: 1.3fr 1.6fr 1.4fr 1fr auto; padding: 11px 18px; font-size: 10px; letter-spacing: 0.1em"
			>
				<span>Provider</span><span>Subject</span><span>Identity</span><span>Last login</span><span></span>
			</div>

			{#if data.identities.length === 0}
				<div class="text-ink-mute text-sm" style="padding: 20px 18px">
					{data.error ? 'Could not load identities.' : 'No identities wired yet — wire one above.'}
				</div>
			{:else}
				{#each data.identities as it, i (it.id)}
					<div
						class="grid items-center gap-4 min-w-[600px] {i < data.identities.length - 1
							? 'border-paper-edge border-b'
							: ''}"
						style="grid-template-columns: 1.3fr 1.6fr 1.4fr 1fr auto; padding: 13px 18px"
					>
						<div class="flex items-center gap-2" style="min-width: 0">
							<span class="kanji text-accent text-center" style="font-size: 15px; width: 20px">{providerKanji(it.provider)}</span>
							<DojoChip toneClass="text-ink-soft">{providerLabel(it.provider)}</DojoChip>
						</div>
						<span class="mono text-ink truncate text-xs" title={it.subject}>{it.subject}</span>
						<div style="min-width: 0">
							<div class="text-ink truncate text-sm">{it.display_name ?? '—'}</div>
							<div class="mono text-ink-faint truncate text-xs">{it.email ?? ''}</div>
						</div>
						<span class="mono text-ink-faint text-xs">{relativeAge(it.last_login_at)}</span>
						<button
							type="button"
							onclick={() => remove(it.id)}
							disabled={busyId === it.id}
							class="border-paper-edge text-ink-soft inline-flex items-center gap-1 rounded-md border text-xs"
							style="padding: 5px 10px; cursor: pointer; background: transparent"
							style:opacity={busyId === it.id ? 0.5 : 1}
							aria-label="Remove identity {it.subject}"
						>
							{busyId === it.id ? 'Removing…' : 'Remove'}
						</button>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
