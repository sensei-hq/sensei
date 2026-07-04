<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import { SENSEI_ROLES, type DaemonChain, type SenseiRole } from '$lib/setup/contracts.js';
  import { ROLE_META } from '$lib/setup/role-meta.js';

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

      <div data-testid="role-detail">
        <!-- Role heading — big kanji + display title + hint -->
        <div class="flex items-baseline gap-2 mb-1">
          <span class="kanji text-[22px] text-accent">{activeMeta.kanji}</span>
          <h3 class="display text-[17px] font-normal m-0">{activeMeta.label}</h3>
        </div>
        <p class="text-[13px] text-ink-soft m-0 mb-4">{activeMeta.hint}</p>

        <!-- Split: priority list (left) + available picker (right) -->
        <div class="grid grid-cols-[1fr_280px] gap-3">
          <!-- Priority list — chain's models, primary marked -->
          <div class="p-3 rounded-md bg-paper border border-paper-edge min-h-[220px]">
            <p class="text-xs uppercase tracking-wider text-ink-soft m-0 mb-2">Priority</p>
            {#if !activePicked || activePicked.models.length === 0}
              <div class="text-center text-[13px] text-ink-faint italic py-6">
                {activePicked
                  ? 'This chain has no models — pick a different chain →'
                  : 'No chain assigned — pick one from the right →'}
              </div>
            {:else}
              <ol class="list-none m-0 p-0 flex flex-col gap-1" data-testid={`chain-models-${active}`}>
                {#each activePicked.models as m (m.sequenceOrder)}
                  {@const primary = m.sequenceOrder === 1}
                  <li
                    class="grid grid-cols-[22px_28px_1fr_auto] gap-2 items-center py-2 px-2 rounded"
                    class:bg-ink={primary}
                    class:text-paper={primary}
                    class:bg-paper-mute={!primary}
                    class:text-ink={!primary}
                  >
                    <span class="font-mono text-xs opacity-60 text-center">{m.sequenceOrder}</span>
                    <span class="kanji text-[13px]" class:text-paper={primary} class:text-accent={!primary}>◆</span>
                    <div class="min-w-0">
                      <div class="text-[13px] font-mono truncate">{m.modelName}</div>
                      <div class="text-xs opacity-60 truncate">
                        {m.routerName.slice(0, 8)}{primary ? ' · PRIMARY' : ''}
                      </div>
                    </div>
                  </li>
                {/each}
              </ol>
            {/if}
          </div>

          <!-- Available picker — other chains user could swap in -->
          <div class="p-3 rounded-md bg-paper-mute border border-paper-edge min-h-[220px]">
            <p class="text-xs uppercase tracking-wider text-ink-soft m-0 mb-2">Available chains</p>
            {#if activeOptions.length === 0}
              <div class="text-center text-[13px] text-ink-faint italic py-4">
                No chain with capability {activeMeta.capabilities.join('/')} yet.
              </div>
            {:else}
              <div class="flex flex-col gap-1" data-testid={`chain-picker-list-${active}`}>
                {#each activeOptions as opt (opt.id)}
                  {@const selected = opt.id === activePickedId}
                  <button
                    type="button"
                    class="grid grid-cols-[1fr_auto] items-center gap-1 py-2 px-2 rounded border-none bg-transparent cursor-pointer text-left text-inherit hover:bg-paper"
                    class:font-medium={selected}
                    data-testid={`chain-opt-${active}-${opt.id}`}
                    onclick={() => pickChain(active, selected ? null : opt.id)}
                  >
                    <div class="min-w-0">
                      <div class="text-[13px] truncate">{opt.name}</div>
                      <div class="text-xs text-ink-soft font-mono">{opt.capability} · {opt.models.length} model{opt.models.length === 1 ? '' : 's'}</div>
                    </div>
                    <span class="text-[13px] {selected ? 'text-accent' : 'text-ink-soft'}">{selected ? '✓' : '+'}</span>
                  </button>
                {/each}
              </div>
            {/if}
            <p class="text-xs text-ink-faint mt-3 italic">
              Chain-model reorder/add coming later.
            </p>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
