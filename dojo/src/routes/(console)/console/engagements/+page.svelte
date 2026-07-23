<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { resolve } from '$app/paths';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import {
		bindEngagementProject,
		createEngagement,
		deleteEngagement,
		DojoApiError,
		updateEngagement,
		type Engagement,
		type NewEngagementBody
	} from '$lib/client-data';
	import {
		bindingsSummary,
		engagementStatusLabel,
		engagementStatusToneClass
	} from '$lib/client-view';
	import { relativeAge } from '$lib/admin-view';
	import DojoJoinEmpty from '$lib/components/DojoJoinEmpty.svelte';
	import { requireTenant } from '$lib/org-guard';

	// Engagements (spec: define engagement · mockup DojoClients "Engagements"
	// registry, dojo-console.jsx l.807). Lists the tenant's client engagements
	// (data.engagements from list_engagements) and lets a lead register
	// (POST …/engagements), edit / close (PATCH), bind a project (POST …/bind), or
	// delete (DELETE) one. Confidentiality is enforced universally at the pipeline
	// strip — engagements are routing only, never a per-client posture. All
	// mutations are gated by the backend LEAD floor; a 403 (non-lead) surfaces in
	// the banner rather than crashing.
	let { data } = $props();

	let actionError = $state<string | null>(null);
	let busyId = $state<string | null>(null);

	// Register form (mockup "+ Register"): the minimum the POST body needs.
	let showRegister = $state(false);
	let formClient = $state('');
	let formDescription = $state('');
	let formStarts = $state('');
	let registering = $state(false);

	const canRegister = $derived(formClient.trim().length > 0 && !registering);

	async function register() {
		if (!canRegister) return;
		registering = true;
		actionError = null;
		const body: NewEngagementBody = { client: formClient.trim() };
		if (formDescription.trim()) body.description = formDescription.trim();
		if (formStarts) body.starts_on = formStarts;
		try {
			await createEngagement(requireTenant(data.tenantKey), body, { fetch, accessToken: data.accessToken });
			formClient = '';
			formDescription = '';
			formStarts = '';
			showRegister = false;
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not register the engagement';
		} finally {
			registering = false;
		}
	}

	// Close an active engagement (retains it for the audit trail — spec: closing
	// is `status = ended`, delete is a hard remove).
	async function close(eng: Engagement) {
		busyId = eng.id;
		actionError = null;
		try {
			await updateEngagement(
				requireTenant(data.tenantKey),
				eng.id,
				{ status: 'ended' },
				{ fetch, accessToken: data.accessToken }
			);
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not close the engagement';
		} finally {
			busyId = null;
		}
	}

	async function remove(eng: Engagement) {
		busyId = eng.id;
		actionError = null;
		try {
			await deleteEngagement(requireTenant(data.tenantKey), eng.id, { fetch, accessToken: data.accessToken });
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not delete the engagement';
		} finally {
			busyId = null;
		}
	}

	// Bind-a-project inline row (POST …/{id}/bind). Only one engagement's binder is
	// open at a time; the project id is required.
	let bindingId = $state<string | null>(null);
	let bindProjectId = $state('');
	let bindName = $state('');

	function openBinder(eng: Engagement) {
		bindingId = eng.id;
		bindProjectId = '';
		bindName = '';
	}

	async function bind(eng: Engagement) {
		if (bindProjectId.trim().length === 0) return;
		busyId = eng.id;
		actionError = null;
		try {
			await bindEngagementProject(
				requireTenant(data.tenantKey),
				eng.id,
				{ project_id: bindProjectId.trim(), name: bindName.trim() || undefined },
				{ fetch, accessToken: data.accessToken }
			);
			bindingId = null;
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not bind the project';
		} finally {
			busyId = null;
		}
	}

	function auditHref(id: string): string {
		return resolve('/(console)/console/engagements/[id]', { id });
	}
</script>

<svelte:head>
	<title>Engagements · Dōjō console</title>
</svelte:head>

{#if data.noMembership}
	<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
		<DojoJoinEmpty what="engagements" />
	</div>
{:else}
<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="客"
		eyebrow="Trust · clients"
		title="Engagements"
		sub="Register a client engagement so its work routes correctly — engagements are routing only. Confidentiality is enforced universally at the pipeline strip: the lesson travels, client, repo and source never do. There are no per-client postures to configure here."
	>
		{#snippet right()}
			<button
				type="button"
				onclick={() => (showRegister = !showRegister)}
				class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
				style="padding: 8px 12px; cursor: pointer; border: none"
			>
				<span class="kanji text-xs">客</span>
				{showRegister ? 'Close' : 'Register engagement'}
			</button>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 24px">
		{#if data.error}
			<ConsoleBanner>
				Live engagements are unavailable.
				<span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}
		{#if actionError}
			<ConsoleBanner>
				Action failed. <span class="mono text-ink-mute text-xs">{actionError}</span>
			</ConsoleBanner>
		{/if}

		<!-- The universal-strip floor (mockup 盾): no per-client override lives here. -->
		<div
			class="bg-paper-soft border-paper-edge border-accent flex items-start gap-3 rounded-xl border"
			style="border-left-width: 3px; padding: 12px 16px; margin: 18px 0"
		>
			<span class="kanji text-accent text-base" style="line-height: 1.2">盾</span>
			<div class="text-ink-soft text-xs" style="line-height: 1.55">
				<b class="text-ink font-semibold">One model for every client.</b> Standards, patterns and the
				memory behind them travel upstream; client, repo, identifiers and source are always dropped. If
				a lesson can't stand without identifying context it's dropped — never weakened, never queued.
			</div>
		</div>

		{#if showRegister}
			<div
				class="bg-paper-soft border-paper-edge rounded-xl border"
				style="padding: 16px 16px; margin-bottom: 18px"
			>
				<div
					class="text-ink-mute font-semibold uppercase text-xs" style="letter-spacing: 0.1em; margin-bottom: 12px"
				>
					Register an engagement
				</div>
				<div class="grid gap-3" style="grid-template-columns: 1.2fr 1.6fr 1fr auto; align-items: end">
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Client</span>
						<input
							class="bg-paper border-paper-edge text-ink rounded-md border text-sm"
							style="padding: 8px 8px"
							bind:value={formClient}
							placeholder="Globex"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Description (optional)</span>
						<input
							class="bg-paper border-paper-edge text-ink rounded-md border text-sm"
							style="padding: 8px 8px"
							bind:value={formDescription}
							placeholder="lumen-auth · billing"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Starts on (optional)</span>
						<input
							type="date"
							class="bg-paper border-paper-edge text-ink rounded-md border text-sm"
							style="padding: 8px 8px"
							bind:value={formStarts}
						/>
					</label>
					<button
						type="button"
						onclick={register}
						disabled={!canRegister}
						class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
						style="padding: 8px 12px; cursor: pointer; border: none"
						style:opacity={canRegister ? 1 : 0.5}
					>
						{registering ? 'Registering…' : 'Register'}
					</button>
				</div>
			</div>
		{/if}

		<div class="bg-paper-soft border-paper-edge overflow-x-auto rounded-xl border">
			<div
				class="border-paper-edge text-ink-mute grid gap-4 border-b font-semibold uppercase min-w-[600px] text-xs" style="grid-template-columns: 1.4fr 1.6fr 0.9fr 0.9fr auto; padding: 12px 16px; letter-spacing: 0.1em"
			>
				<span>Client</span><span>Projects</span><span>Status</span><span>Updated</span><span
					>Actions</span
				>
			</div>

			{#if data.engagements.length === 0}
				<div class="text-ink-mute text-sm" style="padding: 16px 16px">
					{data.error
						? 'Could not load engagements.'
						: 'No engagements yet — register one above.'}
				</div>
			{:else}
				{#each data.engagements as eng, i (eng.id)}
					<div
						class="grid items-center gap-4 min-w-[600px] {i < data.engagements.length - 1
							? 'border-paper-edge border-b'
							: ''}"
						style="grid-template-columns: 1.4fr 1.6fr 0.9fr 0.9fr auto; padding: 12px 16px"
					>
						<div class="flex items-center gap-3" style="min-width: 0">
							<span
								class="kanji text-accent flex-shrink-0 text-center text-base" style="width: 18px">客</span
							>
							<div style="min-width: 0">
								<div class="text-ink truncate text-sm">{eng.client}</div>
								{#if eng.description}
									<div class="text-ink-faint truncate text-xs">{eng.description}</div>
								{/if}
							</div>
						</div>
						<span class="mono text-ink-soft truncate text-xs" title={bindingsSummary(eng)}>
							{bindingsSummary(eng)}
						</span>
						<span>
							<DojoChip toneClass={engagementStatusToneClass(eng.status)}>
								{engagementStatusLabel(eng.status)}
							</DojoChip>
						</span>
						<span class="mono text-ink-faint text-xs">{relativeAge(eng.updated_at)}</span>
						<div class="flex items-center gap-2">
							<a
								href={auditHref(eng.id)}
								class="border-paper-edge text-ink-soft inline-flex items-center rounded-md border text-xs no-underline"
								style="padding: 4px 8px; background: transparent">Audit</a
							>
							<button
								type="button"
								onclick={() => openBinder(eng)}
								disabled={busyId === eng.id}
								class="border-paper-edge text-ink-soft inline-flex items-center rounded-md border text-xs"
								style="padding: 4px 8px; cursor: pointer; background: transparent"
								style:opacity={busyId === eng.id ? 0.5 : 1}>Bind</button
							>
							{#if eng.status === 'active'}
								<button
									type="button"
									onclick={() => close(eng)}
									disabled={busyId === eng.id}
									class="border-paper-edge text-ink-soft inline-flex items-center rounded-md border text-xs"
									style="padding: 4px 8px; cursor: pointer; background: transparent"
									style:opacity={busyId === eng.id ? 0.5 : 1}
									>{busyId === eng.id ? 'Closing…' : 'Close'}</button
								>
							{/if}
							<button
								type="button"
								onclick={() => remove(eng)}
								disabled={busyId === eng.id}
								class="border-danger-edge text-danger inline-flex items-center rounded-md border text-xs"
								style="padding: 4px 8px; cursor: pointer; background: transparent"
								style:opacity={busyId === eng.id ? 0.5 : 1}
								aria-label="Delete engagement {eng.client}">Delete</button
							>
						</div>
					</div>

					{#if bindingId === eng.id}
						<div
							class="bg-paper border-paper-edge grid items-end gap-3 border-b"
							style="grid-template-columns: 1.2fr 1.2fr auto auto; padding: 12px 16px"
						>
							<label class="flex flex-col gap-1">
								<span class="text-ink-mute text-xs">Project id</span>
								<input
									class="bg-paper-soft border-paper-edge text-ink rounded-md border text-sm"
									style="padding: 8px 8px"
									bind:value={bindProjectId}
									placeholder="p-42"
								/>
							</label>
							<label class="flex flex-col gap-1">
								<span class="text-ink-mute text-xs">Name (optional)</span>
								<input
									class="bg-paper-soft border-paper-edge text-ink rounded-md border text-sm"
									style="padding: 8px 8px"
									bind:value={bindName}
									placeholder="ledger-core"
								/>
							</label>
							<button
								type="button"
								onclick={() => bind(eng)}
								disabled={bindProjectId.trim().length === 0 || busyId === eng.id}
								class="bg-ink text-on-primary inline-flex items-center rounded-lg text-xs font-medium"
								style="padding: 8px 12px; cursor: pointer; border: none"
								style:opacity={bindProjectId.trim().length === 0 || busyId === eng.id ? 0.5 : 1}
								>Bind project</button
							>
							<button
								type="button"
								onclick={() => (bindingId = null)}
								class="text-ink-mute inline-flex items-center text-xs"
								style="padding: 8px 4px; cursor: pointer; background: transparent; border: none"
								>Cancel</button
							>
						</div>
					{/if}
				{/each}
			{/if}
		</div>
	</div>
</div>
{/if}
