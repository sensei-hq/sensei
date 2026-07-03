<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import { SENSEI_ROLES, type DaemonChain, type SenseiRole } from '$lib/setup/contracts.js';

  const ROLE_META: Record<SenseiRole, { kanji: string; label: string; hint: string; capabilities: string[] }> = {
    inference: {
      kanji: '推',
      label: 'Inference',
      hint: 'insights, actions, and recommendations from sessions + memory',
      capabilities: ['reasoning'],
    },
    consolidation: {
      kanji: '洞',
      label: 'Consolidation',
      hint: 'merge memories, detect conflicts, propose scope updates',
      capabilities: ['reasoning', 'summarize'],
    },
    embedding: {
      kanji: '印',
      label: 'Embedding',
      hint: 'index sessions, memories, and code refs for retrieval',
      capabilities: ['embed'],
    },
    voice: {
      kanji: '話',
      label: 'Voice',
      hint: 'observatory speech (optional — leave unassigned to keep quiet)',
      capabilities: ['audio', 'chat'],
    },
  };

  let loading = $state(true);
  let error = $state<string | null>(null);
  let active = $state<SenseiRole>('inference');

  const chains = $derived(wizardState.assignments.chains);
  const pending = $derived(wizardState.assignments.pendingAssignments);
  const activeMeta = $derived(ROLE_META[active]);
  const activePickedId = $derived(pending[active]);
  const activePicked = $derived(chainById(activePickedId));
  const activeOptions = $derived(pickableChainsForRole(active));

  // Only chains whose role is either unassigned or already this role are
  // pickable — you can't take a role another chain owns without clearing
  // that one first (the wizard commit handles the ordering).
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

      <div class="p-6 bg-paper-mute border border-paper-mute rounded-lg" data-testid="role-detail">
        <div class="flex items-baseline gap-3 mb-2">
          <span class="kanji text-3xl text-accent">{activeMeta.kanji}</span>
          <h3 class="text-lg font-medium m-0">{activeMeta.label}</h3>
        </div>
        <p class="text-sm text-ink-mute m-0 mb-5">{activeMeta.hint}</p>

        <div class="mb-5 flex items-center gap-3">
          <label for="chain-picker" class="text-xs text-ink-mute uppercase tracking-wide">Chain</label>
          <select
            id="chain-picker"
            class="text-sm px-3 py-2 border border-paper-edge rounded-md bg-paper-soft text-ink cursor-pointer font-mono"
            data-testid={`chain-picker-${active}`}
            value={activePickedId ?? ''}
            onchange={(e) => pickChain(active, e.currentTarget.value || null)}
          >
            <option value="">— none —</option>
            {#each activeOptions as opt (opt.id)}
              <option value={opt.id}>{opt.name} · {opt.capability}</option>
            {/each}
          </select>
          {#if activeOptions.length === 0}
            <span class="text-xs text-warning">No chain with capability {activeMeta.capabilities.join('/')} yet.</span>
          {/if}
        </div>

        {#if activePicked}
          <div>
            <p class="text-xs text-ink-mute uppercase tracking-wide m-0 mb-2">Models in this chain</p>
            {#if activePicked.models.length === 0}
              <p class="text-sm text-ink-soft italic">Chain has no models — pick one, or configure this chain first.</p>
            {:else}
              <ol class="list-none m-0 p-0 flex flex-col gap-1.5" data-testid={`chain-models-${active}`}>
                {#each activePicked.models as m (m.sequenceOrder)}
                  <li class="grid grid-cols-[24px_1fr_auto] gap-3 items-center px-3 py-2 border border-paper-edge rounded-md bg-paper-soft">
                    <span class="text-xs font-mono text-ink-mute">{m.sequenceOrder}</span>
                    <span class="text-sm font-mono">{m.modelName}</span>
                    <span class="text-xs font-mono text-ink-mute">{m.routerName.slice(0, 8)}</span>
                  </li>
                {/each}
              </ol>
            {/if}
          </div>
        {:else}
          <p class="text-sm text-ink-soft italic">No chain assigned to this role. Pick one above to enable this workflow.</p>
        {/if}
      </div>
    </div>
  {/if}
</div>
