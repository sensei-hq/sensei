<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import {
		deletePolicy,
		DojoApiError,
		patchPolicy,
		upsertPolicy,
		type UpsertPolicyBody
	} from '$lib/admin-data';
	import {
		attributionLabel,
		attributionToneClass,
		ATTRIBUTION_MODES,
		retentionLabel
	} from '$lib/admin-view';

	// Policies (spec: the policy grid CRUD). Lists the tenant's per-scope policies
	// (data.policies from list_policies) and lets an admin add/edit-by-scope a
	// policy (POST …/policies upserts on scope_key so an edit takes effect on the
	// next batch), retune attribution inline (PATCH …/policies/{id}), or remove a
	// row (DELETE …/policies/{id}). Gated by the backend ADMIN floor.
	let { data } = $props();

	let actionError = $state<string | null>(null);
	let busyId = $state<string | null>(null);

	async function changeAttribution(id: string, attribution_default: string) {
		busyId = id;
		actionError = null;
		try {
			await patchPolicy(data.tenantKey, id, { attribution_default }, {
				fetch,
				accessToken: data.accessToken
			});
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not update the policy';
		} finally {
			busyId = null;
		}
	}

	async function remove(id: string) {
		busyId = id;
		actionError = null;
		try {
			await deletePolicy(data.tenantKey, id, { fetch, accessToken: data.accessToken });
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not remove the policy';
		} finally {
			busyId = null;
		}
	}

	let showAdd = $state(false);
	let formScope = $state('');
	let formAttribution = $state('named');
	let formRetention = $state('');
	let adding = $state(false);

	const canAdd = $derived(formScope.trim().length > 0 && !adding);

	async function add() {
		if (!canAdd) return;
		adding = true;
		actionError = null;
		const body: UpsertPolicyBody = {
			scope_key: formScope.trim(),
			attribution_default: formAttribution
		};
		const retention = Number.parseInt(formRetention, 10);
		if (Number.isFinite(retention)) body.retention_days = retention;
		try {
			await upsertPolicy(data.tenantKey, body, { fetch, accessToken: data.accessToken });
			formScope = '';
			formRetention = '';
			showAdd = false;
			await invalidateAll();
		} catch (e) {
			actionError = e instanceof DojoApiError ? e.message : 'could not save the policy';
		} finally {
			adding = false;
		}
	}
</script>

<svelte:head>
	<title>Policies · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="規"
		eyebrow="Org · policy"
		title="Policies"
		sub="Attribution defaults, confidentiality rules, and retention windows per scope. A policy edit upserts on its scope and takes effect on the next batch."
	>
		{#snippet right()}
			<button
				type="button"
				onclick={() => (showAdd = !showAdd)}
				class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
				style="padding: 8px 12px; cursor: pointer; border: none"
			>
				<span class="kanji text-xs">規</span>
				{showAdd ? 'Close' : 'Add policy'}
			</button>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 24px">
		{#if data.error}
			<ConsoleBanner>
				Live policies are unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}
		{#if actionError}
			<ConsoleBanner>
				Action failed. <span class="mono text-ink-mute text-xs">{actionError}</span>
			</ConsoleBanner>
		{/if}

		{#if showAdd}
			<div class="bg-paper-soft border-paper-edge rounded-xl border" style="padding: 16px 16px; margin: 18px 0">
				<div class="text-ink-mute font-semibold uppercase text-xs" style="letter-spacing: 0.1em; margin-bottom: 12px">
					Add or edit a scope policy
				</div>
				<div class="grid gap-3" style="grid-template-columns: 1.6fr 1.2fr 1fr auto; align-items: end">
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Scope key</span>
						<input class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 8px 8px" bind:value={formScope} placeholder="Team · Payments" />
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Attribution default</span>
						<select class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 8px 8px" bind:value={formAttribution}>
							{#each ATTRIBUTION_MODES as m (m)}
								<option value={m}>{attributionLabel(m)}</option>
							{/each}
						</select>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-ink-mute text-xs">Retention (days)</span>
						<input class="bg-paper border-paper-edge text-ink rounded-md border text-sm" style="padding: 8px 8px" bind:value={formRetention} placeholder="365" inputmode="numeric" />
					</label>
					<button
						type="button"
						onclick={add}
						disabled={!canAdd}
						class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
						style="padding: 8px 12px; cursor: pointer; border: none"
						style:opacity={canAdd ? 1 : 0.5}
					>
						{adding ? 'Saving…' : 'Save'}
					</button>
				</div>
			</div>
		{/if}

		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border" style="margin-top: 18px">
			<div
				class="border-paper-edge text-ink-mute grid gap-4 border-b font-semibold uppercase text-xs" style="grid-template-columns: 1.8fr 1.4fr 1fr 0.9fr auto; padding: 12px 16px; letter-spacing: 0.1em"
			>
				<span>Scope</span><span>Attribution</span><span>Retention</span><span>Updated</span><span></span>
			</div>

			{#if data.policies.length === 0}
				<div class="text-ink-mute text-sm" style="padding: 16px 16px">
					{data.error ? 'Could not load policies.' : 'No policies yet — a scope inherits its parent until one is set.'}
				</div>
			{:else}
				{#each data.policies as p, i (p.id)}
					<div
						class="grid items-center gap-4 {i < data.policies.length - 1 ? 'border-paper-edge border-b' : ''}"
						style="grid-template-columns: 1.8fr 1.4fr 1fr 0.9fr auto; padding: 12px 16px"
					>
						<div class="flex items-center gap-2" style="min-width: 0">
							<span class="kanji text-ink-mute text-center text-sm" style="width: 18px">層</span>
							<span class="text-ink truncate text-sm">{p.scope_key}</span>
						</div>
						<div class="flex items-center gap-2">
							<select
								value={p.attribution_default}
								disabled={busyId === p.id}
								onchange={(e) => changeAttribution(p.id, e.currentTarget.value)}
								class="bg-paper border-paper-edge rounded-md border text-sm {attributionToneClass(p.attribution_default)}"
								style="padding: 4px 8px; cursor: pointer"
								aria-label="Attribution for {p.scope_key}"
							>
								{#each ATTRIBUTION_MODES as m (m)}
									<option value={m}>{attributionLabel(m)}</option>
								{/each}
							</select>
							{#if busyId === p.id}
								<span class="mono text-ink-faint text-xs">saving…</span>
							{/if}
						</div>
						<span>
							<DojoChip toneClass={p.retention_days == null ? 'text-ink-mute' : 'text-ink-soft'}>
								{retentionLabel(p.retention_days)}
							</DojoChip>
						</span>
						<span class="mono text-ink-faint text-xs">{p.updated_at.slice(0, 10)}</span>
						<button
							type="button"
							onclick={() => remove(p.id)}
							disabled={busyId === p.id}
							class="border-paper-edge text-ink-soft inline-flex items-center gap-1 rounded-md border text-xs"
							style="padding: 4px 8px; cursor: pointer; background: transparent"
							style:opacity={busyId === p.id ? 0.5 : 1}
							aria-label="Remove policy {p.scope_key}"
						>
							Remove
						</button>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
