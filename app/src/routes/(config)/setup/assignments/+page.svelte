<script lang="ts">
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import { SENSEI_ROLES, type DaemonChain, type SenseiRole } from '$lib/setup/contracts.js';
  import { ROLE_META } from '$lib/setup/role-meta.js';

  let loading = $state(true);
  let error = $state<string | null>(null);
  let active = $state<SenseiRole>('inference');

  // Per-row busy so a slow ▲/▼/× on one member doesn't block another.
  let busyMember = $state<Record<string, 'up' | 'down' | 'remove' | null>>({});
  // Add-model spinner keyed by the "modelId::routerId" tuple.
  let addingKey = $state<string | null>(null);

  // Available models for the currently-selected chain. Refetched
  // whenever the picked chain changes so the picker stays in sync.
  let available = $state<Array<{
    modelId: string; modelName: string; fullName: string;
    routerId: string; routerName: string;
  }>>([]);
  let availableLoading = $state(false);

  const chains = $derived(wizardState.assignments.chains);
  const pending = $derived(wizardState.assignments.pendingAssignments);
  const activeMeta = $derived(ROLE_META[active]);
  const activePickedId = $derived(pending[active]);
  const activePicked = $derived(chainById(activePickedId));
  const activeOptions = $derived(pickableChainsForRole(active));

  function pickableChainsForRole(role: SenseiRole): DaemonChain[] {
    const meta = ROLE_META[role];
    return chains.filter(c =>
      c.isActive
      && meta.capabilities.includes(c.capability)
      && (c.role == null || c.role === role),
    );
  }

  function pickChain(role: SenseiRole, chainId: string | null): void {
    wizardState.assignments.pendingAssignments = {
      ...pending,
      [role]: chainId,
    };
  }

  function chainById(id: string | null): DaemonChain | undefined {
    if (!id) return undefined;
    return chains.find(c => c.id === id);
  }

  // Fetch available models whenever the picked chain changes. Gated on
  // !loading so the initial hydrate doesn't fire it (refreshAssignments
  // triggers this once via `activePickedId` becoming truthy).
  $effect(() => {
    const chainId = activePickedId;
    if (loading || !chainId) { available = []; return; }
    availableLoading = true;
    const api = senseiApi(appState.port);
    api.listAvailableChainModels(chainId)
      .then((r) => { available = r.models; })
      .catch((e) => console.warn('[assignments] listAvailableChainModels failed', e))
      .finally(() => { availableLoading = false; });
  });

  async function refreshChainsAndAvailable(): Promise<void> {
    await wizardState.refreshAssignments();
    const chainId = activePickedId;
    if (!chainId) return;
    availableLoading = true;
    try {
      const r = await senseiApi(appState.port).listAvailableChainModels(chainId);
      available = r.models;
    } finally {
      availableLoading = false;
    }
  }

  async function moveMember(memberId: string, direction: -1 | 1): Promise<void> {
    const chainId = activePickedId;
    if (!chainId) return;
    busyMember[memberId] = direction === -1 ? 'up' : 'down';
    try {
      const res = await senseiApi(appState.port).moveGatewayChainModel(chainId, memberId, direction);
      if (!res.ok) console.warn('[assignments] move failed', res.error);
      await refreshChainsAndAvailable();
    } finally {
      busyMember[memberId] = null;
    }
  }

  async function removeMember(memberId: string): Promise<void> {
    const chainId = activePickedId;
    if (!chainId) return;
    busyMember[memberId] = 'remove';
    try {
      const res = await senseiApi(appState.port).removeGatewayChainModel(chainId, memberId);
      if (!res.ok) console.warn('[assignments] remove failed', res.error);
      await refreshChainsAndAvailable();
    } finally {
      busyMember[memberId] = null;
    }
  }

  async function addMember(modelId: string, routerId: string): Promise<void> {
    const chainId = activePickedId;
    if (!chainId) return;
    addingKey = `${modelId}::${routerId}`;
    try {
      const res = await senseiApi(appState.port).addGatewayChainModel(chainId, modelId, routerId);
      if (!res.ok) console.warn('[assignments] add failed', res.error);
      await refreshChainsAndAvailable();
    } finally {
      addingKey = null;
    }
  }

  onMount(async () => {
    try {
      await wizardState.refreshAssignments();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="max-w-[1040px]">
  <p class="text-sm text-ink-soft leading-normal m-0 mb-6">
    Decide which chain of models handles each reasoning role. Chains are
    ordered — the first model is primary; the rest are fallbacks used if
    the primary is unavailable. Voice is optional.
  </p>

  {#if loading}
    <div data-testid="assignments-loading" class="text-center p-12 bg-paper-mute rounded-lg border border-paper-mute">
      <span class="kanji text-4xl text-accent opacity-20 block mb-4">任</span>
      <p class="text-sm text-ink-soft">Loading chains…</p>
    </div>
  {:else if error}
    <div data-testid="assignments-error" class="mb-6 p-4 rounded-md border border-danger bg-paper-mute">
      <div class="text-sm font-semibold text-danger">Could not load chains</div>
      <div class="text-xs text-ink-mute mt-1 font-mono">{error}</div>
    </div>
  {:else}
    <div class="grid grid-cols-[260px_1fr] gap-6" data-testid="assignments-body">
      <!-- Roles rail -->
      <div class="flex flex-col gap-1" role="tablist" aria-label="Sensei roles" data-testid="role-list">
        {#each SENSEI_ROLES as role (role)}
          {@const meta = ROLE_META[role]}
          {@const pickedId = pending[role]}
          {@const picked = chainById(pickedId)}
          {@const on = active === role}
          <button
            type="button"
            role="tab"
            class="grid grid-cols-[28px_1fr_auto] items-center gap-3 py-3 px-3 rounded-md border cursor-pointer text-left"
            class:bg-ink={on}
            class:text-paper={on}
            class:border-transparent={on}
            class:bg-paper-soft={!on}
            class:text-ink={!on}
            class:border-paper-edge={!on}
            data-testid={`role-tab-${role}`}
            aria-selected={on}
            onclick={() => (active = role)}
          >
            <span class="kanji text-lg" class:text-paper={on} class:text-accent={!on}>{meta.kanji}</span>
            <div class="min-w-0">
              <div class="text-sm">{meta.label}</div>
              <div class="text-xs opacity-70 truncate mt-0.5">
                {picked ? picked.name : '— none —'}
              </div>
            </div>
            <span class="text-xs opacity-60 font-mono">{picked ? picked.models.length : 0}</span>
          </button>
        {/each}
      </div>

      <!-- Role detail -->
      <div data-testid="role-detail">
        <!-- Role heading -->
        <div class="flex items-baseline gap-2 mb-1">
          <span class="kanji text-[22px] text-accent">{activeMeta.kanji}</span>
          <h3 class="display text-[17px] font-normal m-0">{activeMeta.label}</h3>
        </div>
        <p class="text-[13px] text-ink-soft m-0 mb-3">{activeMeta.hint}</p>

        <!-- Chain selector — small inline picker -->
        <div class="flex items-center gap-2 mb-4">
          <span class="text-xs uppercase tracking-wider text-ink-soft">Chain</span>
          <select
            class="text-xs px-2 py-1 border border-paper-edge rounded bg-paper-soft text-ink cursor-pointer font-mono"
            data-testid={`chain-picker-${active}`}
            value={activePickedId ?? ''}
            onchange={(e) => pickChain(active, e.currentTarget.value || null)}
          >
            <option value="">— none —</option>
            {#each activeOptions as opt (opt.id)}
              <option value={opt.id}>{opt.name}</option>
            {/each}
          </select>
          {#if activeOptions.length === 0}
            <span class="text-xs text-warning">No chain with capability {activeMeta.capabilities.join('/')} yet.</span>
          {/if}
        </div>

        <!-- Priority list + Available models split -->
        <div class="grid grid-cols-[1fr_280px] gap-3">
          <!-- Priority list — chain's ordered models with ▲/▼/× -->
          <div class="p-3 rounded-md bg-paper border border-paper-edge min-h-[220px]">
            <p class="text-xs uppercase tracking-wider text-ink-soft m-0 mb-2">Priority</p>
            {#if !activePicked || activePicked.models.length === 0}
              <div class="text-center text-[13px] text-ink-faint italic py-6">
                {activePicked
                  ? 'No models — add one from the right →'
                  : 'No chain assigned — pick one above.'}
              </div>
            {:else}
              <ol class="list-none m-0 p-0 flex flex-col gap-1" data-testid={`chain-models-${active}`}>
                {#each activePicked.models as m, i (m.memberId)}
                  {@const primary = m.sequenceOrder === 1}
                  {@const busy = busyMember[m.memberId]}
                  <li
                    class="grid grid-cols-[22px_28px_1fr_auto_auto_auto] gap-2 items-center py-2 px-2 rounded"
                    class:bg-ink={primary}
                    class:text-paper={primary}
                    class:bg-paper-mute={!primary}
                    class:text-ink={!primary}
                    data-testid={`chain-member-${m.memberId}`}
                  >
                    <span class="font-mono text-xs opacity-60 text-center">{m.sequenceOrder}</span>
                    <span class="kanji text-[13px]" class:text-paper={primary} class:text-accent={!primary}>◆</span>
                    <div class="min-w-0">
                      <div class="text-[13px] font-mono truncate">{m.modelName}</div>
                      <div class="text-xs opacity-60 truncate">
                        {m.routerName.slice(0, 8)}{primary ? ' · PRIMARY' : ''}
                      </div>
                    </div>
                    <button
                      type="button"
                      class="w-6 h-6 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={i === 0 || busy}
                      class:text-paper={primary}
                      class:text-ink-mute={!primary}
                      disabled={i === 0 || !!busy}
                      data-testid={`member-up-${m.memberId}`}
                      onclick={() => moveMember(m.memberId, -1)}
                      title="Move up"
                    >▲</button>
                    <button
                      type="button"
                      class="w-6 h-6 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={i === activePicked.models.length - 1 || busy}
                      class:text-paper={primary}
                      class:text-ink-mute={!primary}
                      disabled={i === activePicked.models.length - 1 || !!busy}
                      data-testid={`member-down-${m.memberId}`}
                      onclick={() => moveMember(m.memberId, 1)}
                      title="Move down"
                    >▼</button>
                    <button
                      type="button"
                      class="w-6 h-6 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={busy}
                      class:text-paper={primary}
                      class:text-ink-mute={!primary}
                      disabled={!!busy}
                      data-testid={`member-remove-${m.memberId}`}
                      onclick={() => removeMember(m.memberId)}
                      title="Remove"
                    >×</button>
                  </li>
                {/each}
              </ol>
            {/if}
          </div>

          <!-- Available models — grouped by router -->
          <div class="p-3 rounded-md bg-paper-mute border border-paper-edge min-h-[220px]">
            <p class="text-xs uppercase tracking-wider text-ink-soft m-0 mb-2">Add model</p>
            {#if !activePicked}
              <div class="text-center text-[13px] text-ink-faint italic py-4">
                Pick a chain to see available models.
              </div>
            {:else if availableLoading}
              <div class="text-center text-[13px] text-ink-faint italic py-4">
                Loading…
              </div>
            {:else if available.length === 0}
              <div class="text-center text-[13px] text-ink-faint italic py-4">
                All matching models are in the chain.
              </div>
            {:else}
              <div class="flex flex-col gap-1" data-testid={`available-list-${active}`}>
                {#each available as opt (opt.modelId + '::' + opt.routerId)}
                  {@const key = `${opt.modelId}::${opt.routerId}`}
                  {@const busy = addingKey === key}
                  <button
                    type="button"
                    class="grid grid-cols-[1fr_auto] items-center gap-1 py-2 px-2 rounded border-none bg-transparent cursor-pointer text-left text-inherit hover:bg-paper"
                    class:opacity-50={busy}
                    disabled={busy}
                    data-testid={`available-add-${opt.modelId}`}
                    onclick={() => addMember(opt.modelId, opt.routerId)}
                  >
                    <div class="min-w-0">
                      <div class="text-[13px] font-mono truncate">{opt.modelName}</div>
                      <div class="text-xs text-ink-soft font-mono truncate">{opt.routerName}</div>
                    </div>
                    <span class="text-[13px] text-ink-soft">+</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
