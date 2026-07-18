<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import {
		createIncident,
		deleteIncident,
		DojoApiError,
		updateIncident,
		type Incident,
		type IncidentSeverity,
		type NewIncidentBody
	} from '$lib/client-data';
	import {
		incidentStatusLabel,
		incidentStatusToneClass,
		INCIDENT_SEVERITIES,
		INCIDENT_STATUSES,
		isIncidentOpen,
		isSlaBreached,
		severityLabel,
		severityToneClass
	} from '$lib/client-view';
	import { clockTime, shortId } from '$lib/admin-view';

	// Incidents (spec: incident handling 警 · contain a near-leak fast). Lists the
	// tenant's confidentiality incidents (data.incidents, worst-severity first) and
	// lets a lead open (POST …/incidents), update severity / status / owner
	// (PATCH), resolve (PATCH resolved: true), or delete (DELETE) one. The open
	// count (`resolved_at is null`) is the done-gate. All mutations are gated by the
	// backend LEAD floor; a 403 surfaces in the banner rather than crashing.
	let { data } = $props();

	let actionError = $state<string | null>(null);
	let busyId = $state<string | null>(null);

	// Open-incident form (mockup "警" incident dashboard).
	let showOpen = $state(false);
	let formTitle = $state('');
	let formSeverity = $state<IncidentSeverity>('medium');
	let formDescription = $state('');
	let opening = $state(false);

	const canOpen = $derived(formTitle.trim().length > 0 && !opening);

	async function openIncident() {
		if (!canOpen) return;
		opening = true;
		actionError = null;
		const body: NewIncidentBody = { title: formTitle.trim(), severity: formSeverity };
		if (formDescription.trim()) body.description = formDescription.trim();
		try {
			await createIncident(data.tenantKey, body, { fetch, accessToken: data.accessToken });
			formTitle = '';
			formDescription = '';
			formSeverity = 'medium';
			showOpen = false;
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not open the incident';
		} finally {
			opening = false;
		}
	}

	async function setSeverity(inc: Incident, severity: string) {
		busyId = inc.id;
		actionError = null;
		try {
			await updateIncident(
				data.tenantKey,
				inc.id,
				{ severity: severity as IncidentSeverity },
				{ fetch, accessToken: data.accessToken }
			);
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not update the severity';
		} finally {
			busyId = null;
		}
	}

	async function setStatus(inc: Incident, status: string) {
		busyId = inc.id;
		actionError = null;
		try {
			// `status = resolved` closes it server-side; open|investigating reopens.
			await updateIncident(
				data.tenantKey,
				inc.id,
				{ status: status as 'open' | 'investigating' | 'resolved' },
				{ fetch, accessToken: data.accessToken }
			);
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not update the status';
		} finally {
			busyId = null;
		}
	}

	async function remove(inc: Incident) {
		busyId = inc.id;
		actionError = null;
		try {
			await deleteIncident(data.tenantKey, inc.id, { fetch, accessToken: data.accessToken });
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not delete the incident';
		} finally {
			busyId = null;
		}
	}
</script>

<svelte:head>
	<title>Incidents · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="警"
		eyebrow="Trust · incidents"
		title="Incident handling"
		sub="Contain a near-leak fast. Open an incident with a severity and owner, work it through investigating to resolved, and hold the trail. Open incidents past their SLA are flagged so nothing slips."
	>
		{#snippet right()}
			<div class="flex items-center gap-3">
				<span class="mono text-ink-mute text-xs">
					<b class={data.openCount > 0 ? 'text-danger' : 'text-success'}>{data.openCount}</b> open
				</span>
				<button
					type="button"
					onclick={() => (showOpen = !showOpen)}
					class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
					style="padding: 8px 13px; cursor: pointer; border: none"
				>
					<span class="kanji" style="font-size: 12px">警</span>
					{showOpen ? 'Close' : 'Open incident'}
				</button>
			</div>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 28px">
		{#if data.error}
			<ConsoleBanner>
				Live incidents are unavailable.
				<span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}
		{#if actionError}
			<ConsoleBanner>
				Action failed. <span class="mono text-ink-mute text-xs">{actionError}</span>
			</ConsoleBanner>
		{/if}

		{#if showOpen}
			<div
				class="bg-paper-soft border-paper-edge rounded-xl border"
				style="padding: 16px 18px; margin: 18px 0"
			>
				<div
					class="text-ink-mute font-semibold uppercase"
					style="font-size: 10px; letter-spacing: 0.1em; margin-bottom: 12px"
				>
					Open an incident
				</div>
				<div class="grid gap-3" style="grid-template-columns: 1.6fr 0.8fr 1.4fr auto; align-items: end">
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Title</span>
						<input
							class="bg-paper border-paper-edge text-ink rounded-md border text-sm"
							style="padding: 7px 9px"
							bind:value={formTitle}
							placeholder="Near-leak in shared example"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Severity</span>
						<select
							class="bg-paper border-paper-edge rounded-md border text-sm {severityToneClass(
								formSeverity
							)}"
							style="padding: 7px 9px"
							bind:value={formSeverity}
						>
							{#each INCIDENT_SEVERITIES as s (s)}
								<option value={s}>{severityLabel(s)}</option>
							{/each}
						</select>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Description (optional)</span>
						<input
							class="bg-paper border-paper-edge text-ink rounded-md border text-sm"
							style="padding: 7px 9px"
							bind:value={formDescription}
							placeholder="what happened"
						/>
					</label>
					<button
						type="button"
						onclick={openIncident}
						disabled={!canOpen}
						class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
						style="padding: 8px 13px; cursor: pointer; border: none"
						style:opacity={canOpen ? 1 : 0.5}
					>
						{opening ? 'Opening…' : 'Open'}
					</button>
				</div>
			</div>
		{/if}

		<div class="bg-paper-soft border-paper-edge overflow-x-auto rounded-xl border" style="margin-top: 18px">
			<div
				class="border-paper-edge text-ink-mute grid gap-3 border-b font-semibold uppercase min-w-[660px]"
				style="grid-template-columns: 0.9fr 1.6fr 1.1fr 1fr 1fr auto; padding: 11px 18px; font-size: 10px; letter-spacing: 0.1em"
			>
				<span>Severity</span><span>Incident</span><span>Status</span><span>SLA</span><span>Owner</span
				><span>Actions</span>
			</div>

			{#if data.incidents.length === 0}
				<div class="text-ink-mute text-sm" style="padding: 20px 18px">
					{data.error ? 'Could not load incidents.' : 'No incidents — nothing to contain.'}
				</div>
			{:else}
				{#each data.incidents as inc, i (inc.id)}
					{@const breached = isSlaBreached(inc)}
					<div
						class="grid items-center gap-3 min-w-[660px] {i < data.incidents.length - 1
							? 'border-paper-edge border-b'
							: ''} {breached ? 'bg-danger-soft' : ''}"
						style="grid-template-columns: 0.9fr 1.6fr 1.1fr 1fr 1fr auto; padding: 12px 18px"
					>
						<div class="flex items-center gap-2">
							<select
								value={inc.severity}
								disabled={busyId === inc.id}
								onchange={(e) => setSeverity(inc, e.currentTarget.value)}
								class="bg-paper border-paper-edge rounded-md border text-xs {severityToneClass(
									inc.severity
								)}"
								style="padding: 3px 6px; cursor: pointer"
								aria-label="Severity for {inc.title}"
							>
								{#each INCIDENT_SEVERITIES as s (s)}
									<option value={s}>{severityLabel(s)}</option>
								{/each}
							</select>
						</div>
						<div style="min-width: 0">
							<div class="text-ink truncate text-sm" title={inc.title}>{inc.title}</div>
							{#if inc.description}
								<div class="text-ink-faint truncate text-xs">{inc.description}</div>
							{/if}
						</div>
						<div class="flex items-center gap-2">
							<select
								value={inc.status}
								disabled={busyId === inc.id}
								onchange={(e) => setStatus(inc, e.currentTarget.value)}
								class="bg-paper border-paper-edge rounded-md border text-xs {incidentStatusToneClass(
									inc.status
								)}"
								style="padding: 3px 6px; cursor: pointer"
								aria-label="Status for {inc.title}"
							>
								{#each INCIDENT_STATUSES as s (s)}
									<option value={s}>{incidentStatusLabel(s)}</option>
								{/each}
							</select>
						</div>
						<div class="flex items-center gap-1" style="min-width: 0">
							{#if inc.sla_due_at}
								<span class="mono text-xs {breached ? 'text-danger' : 'text-ink-faint'}"
									>{clockTime(inc.sla_due_at)}</span
								>
								{#if breached}
									<DojoChip toneClass="text-danger">past SLA</DojoChip>
								{/if}
							{:else}
								<span class="mono text-ink-faint text-xs">—</span>
							{/if}
						</div>
						<span class="mono text-ink-soft truncate text-xs" title={inc.owner_id ?? ''}
							>{inc.owner_id ? shortId(inc.owner_id, 10) : '—'}</span
						>
						<div class="flex items-center gap-2">
							{#if isIncidentOpen(inc)}
								<button
									type="button"
									onclick={() => setStatus(inc, 'resolved')}
									disabled={busyId === inc.id}
									class="border-success-edge text-success inline-flex items-center rounded-md border text-xs"
									style="padding: 5px 10px; cursor: pointer; background: transparent"
									style:opacity={busyId === inc.id ? 0.5 : 1}
									>{busyId === inc.id ? 'Resolving…' : 'Resolve'}</button
								>
							{:else}
								<button
									type="button"
									onclick={() => setStatus(inc, 'open')}
									disabled={busyId === inc.id}
									class="border-paper-edge text-ink-soft inline-flex items-center rounded-md border text-xs"
									style="padding: 5px 10px; cursor: pointer; background: transparent"
									style:opacity={busyId === inc.id ? 0.5 : 1}>Reopen</button
								>
							{/if}
							<button
								type="button"
								onclick={() => remove(inc)}
								disabled={busyId === inc.id}
								class="border-danger-edge text-danger inline-flex items-center rounded-md border text-xs"
								style="padding: 5px 10px; cursor: pointer; background: transparent"
								style:opacity={busyId === inc.id ? 0.5 : 1}
								aria-label="Delete incident {inc.title}">Delete</button
							>
						</div>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
