<script lang="ts">
  import { onMount } from 'svelte';
  import { Select } from '@rokkit/ui';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { SENSEI_ROLES, type DaemonChain, type SenseiRole } from '$lib/setup/contracts.js';
  import { ROLE_META } from '$lib/setup/role-meta.js';

  let chains = $state<DaemonChain[]>([]);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  // Role → chain id; daemon truth after each write.
  let assignments = $state<Record<SenseiRole, string | null>>({
    inference: null, consolidation: null, embedding: null, voice: null,
  });
  // Per-role save status so a slow write to one role doesn't hide the
  // outcome for another.
  let saveStatus = $state<Record<SenseiRole, 'idle' | 'saving' | 'saved' | 'error'>>({
    inference: 'idle', consolidation: 'idle', embedding: 'idle', voice: 'idle',
  });
  let saveError = $state<Record<SenseiRole, string | null>>({
    inference: null, consolidation: null, embedding: null, voice: null,
  });
  // Per-member busy so a slow move/remove on one row doesn't disable
  // its neighbours. Keyed by fallback_chain_models.id.
  let memberBusy = $state<Record<string, 'up' | 'down' | 'remove' | null>>({});

  async function loadChains(): Promise<void> {
    const api = senseiApi(appState.port);
    try {
      const result = await api.listGatewayChains();
      chains = result.chains;
      const daemon: Record<SenseiRole, string | null> = {
        inference: null, consolidation: null, embedding: null, voice: null,
      };
      for (const c of chains) {
        if (c.role != null && daemon[c.role] === null) daemon[c.role] = c.id;
      }
      assignments = daemon;
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function optionsFor(role: SenseiRole): DaemonChain[] {
    const meta = ROLE_META[role];
    return chains.filter(c =>
      c.isActive
      && meta.capabilities.includes(c.capability)
      && (c.role == null || c.role === role),
    );
  }

  function chainById(id: string | null): DaemonChain | undefined {
    if (!id) return undefined;
    return chains.find(c => c.id === id);
  }

  async function pick(role: SenseiRole, nextChainId: string | null): Promise<void> {
    const prev = assignments[role];
    if ((nextChainId ?? null) === (prev ?? null)) return;
    const api = senseiApi(appState.port);
    saveStatus[role] = 'saving';
    saveError[role] = null;

    // Two-phase: CLEAR the old chain (frees the role), then SET the new.
    // Same invariant as the wizard commit — otherwise the unique-when-set
    // index rejects the second write.
    if (prev) {
      const cleared = await api.setGatewayChainRole(prev, null);
      if (!cleared.ok) {
        saveStatus[role] = 'error';
        saveError[role] = cleared.error.message;
        return;
      }
    }
    if (nextChainId) {
      const setResult = await api.setGatewayChainRole(nextChainId, role);
      if (!setResult.ok) {
        saveStatus[role] = 'error';
        saveError[role] = setResult.error.message;
        // Roll `assignments` back so the picker reflects daemon truth.
        await loadChains();
        return;
      }
    }
    assignments[role] = nextChainId;
    saveStatus[role] = 'saved';
    setTimeout(() => { if (saveStatus[role] === 'saved') saveStatus[role] = 'idle'; }, 1500);
  }

  async function moveMember(role: SenseiRole, memberId: string, direction: -1 | 1): Promise<void> {
    const chainId = assignments[role];
    if (!chainId) return;
    memberBusy[memberId] = direction === -1 ? 'up' : 'down';
    saveStatus[role] = 'saving';
    try {
      const api = senseiApi(appState.port);
      const res = await api.moveGatewayChainModel(chainId, memberId, direction);
      if (!res.ok) {
        saveStatus[role] = 'error';
        saveError[role] = res.error.message;
        return;
      }
      await loadChains();
      saveStatus[role] = 'saved';
      setTimeout(() => { if (saveStatus[role] === 'saved') saveStatus[role] = 'idle'; }, 1500);
    } finally {
      memberBusy[memberId] = null;
    }
  }

  async function removeMember(role: SenseiRole, memberId: string): Promise<void> {
    const chainId = assignments[role];
    if (!chainId) return;
    memberBusy[memberId] = 'remove';
    saveStatus[role] = 'saving';
    try {
      const api = senseiApi(appState.port);
      const res = await api.removeGatewayChainModel(chainId, memberId);
      if (!res.ok) {
        saveStatus[role] = 'error';
        saveError[role] = res.error.message;
        return;
      }
      await loadChains();
      saveStatus[role] = 'saved';
      setTimeout(() => { if (saveStatus[role] === 'saved') saveStatus[role] = 'idle'; }, 1500);
    } finally {
      memberBusy[memberId] = null;
    }
  }

  onMount(loadChains);
</script>

<div class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg" data-testid="settings-inference">
  <h3 class="text-base m-0 mb-1">Inference</h3>
  <p class="text-sm text-ink-soft m-0 mb-5">
    Which model chain handles each reasoning role. Chains carry an ordered
    fallback list; change them here to try a different mix.
  </p>

  {#if loading}
    <p class="text-sm text-ink-soft">Loading chains…</p>
  {:else if loadError}
    <p class="text-sm text-danger" data-testid="settings-inference-error">Could not load chains: {loadError}</p>
  {:else}
    <div class="flex flex-col divide-y divide-paper-edge">
      {#each SENSEI_ROLES as role (role)}
        {@const meta = ROLE_META[role]}
        {@const currentId = assignments[role]}
        {@const current = chainById(currentId)}
        {@const options = optionsFor(role)}
        {@const status = saveStatus[role]}
        <div class="grid grid-cols-[28px_1fr_auto] gap-3 py-4 items-start" data-testid={`inference-role-${role}`}>
          <span class="kanji text-[22px] text-accent leading-none mt-1">{meta.kanji}</span>
          <div class="min-w-0">
            <div class="text-[13px] text-ink font-medium">{meta.label}</div>
            <div class="text-xs text-ink-soft mt-0.5">{meta.hint}</div>
            {#if current && current.models.length > 0}
              <!-- Chain editor: model chips + ▲/▼/× per member. Adding a
                   new model happens in the wizard's fuller "Available
                   models" picker; here we keep the chain-model editor
                   compact — you can only reorder or remove. -->
              <div class="flex flex-col gap-1 mt-2" data-testid={`inference-chain-members-${role}`}>
                {#each current.models as m, i (m.memberId)}
                  {@const primary = i === 0}
                  {@const busy = memberBusy[m.memberId]}
                  <div
                    class="grid grid-cols-[auto_1fr_auto_auto_auto] items-center gap-2 py-1 px-2 rounded"
                    class:bg-ink={primary}
                    class:text-paper={primary}
                    class:bg-paper-mute={!primary}
                    data-testid={`inference-chain-member-${m.memberId}`}
                  >
                    <span class="font-mono text-xs opacity-70">{i + 1}.</span>
                    <span class="font-mono text-xs truncate">{m.modelName}</span>
                    <button
                      type="button"
                      class="w-5 h-5 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={i === 0 || busy}
                      disabled={i === 0 || !!busy}
                      title="Move up"
                      data-testid={`inference-member-up-${m.memberId}`}
                      onclick={() => moveMember(role, m.memberId, -1)}
                    >▲</button>
                    <button
                      type="button"
                      class="w-5 h-5 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={i === current.models.length - 1 || busy}
                      disabled={i === current.models.length - 1 || !!busy}
                      title="Move down"
                      data-testid={`inference-member-down-${m.memberId}`}
                      onclick={() => moveMember(role, m.memberId, 1)}
                    >▼</button>
                    <button
                      type="button"
                      class="w-5 h-5 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={busy}
                      disabled={!!busy}
                      title="Remove"
                      data-testid={`inference-member-remove-${m.memberId}`}
                      onclick={() => removeMember(role, m.memberId)}
                    >×</button>
                  </div>
                {/each}
              </div>
            {/if}
            {#if status === 'error' && saveError[role]}
              <div class="text-xs text-danger mt-1" data-testid={`inference-role-error-${role}`}>
                {saveError[role]}
              </div>
            {/if}
          </div>
          <div class="flex items-center gap-2">
            <span
              class="text-xs w-14 text-right"
              class:text-ink-mute={status === 'idle'}
              class:text-ink-soft={status === 'saving'}
              class:text-success={status === 'saved'}
              class:text-warning={status === 'error'}
              data-testid={`inference-role-status-${role}`}
            >
              {#if status === 'saving'}saving…
              {:else if status === 'saved'}saved
              {:else if status === 'error'}error
              {/if}
            </span>
            <!-- data-testid on a wrapper: rokkit Select (v1.3.1) doesn't forward
                 arbitrary data-* to the DOM (only `class` reaches the root), so the
                 e2e/test selector must live on the wrapper, not the component. -->
            <div data-testid={`inference-role-picker-${role}`} class="min-w-[160px]">
              <Select
                size="sm"
                items={[{ id: '', name: '— none —' }, ...options]}
                fields={{ label: 'name', value: 'id' }}
                value={currentId ?? ''}
                onchange={(v) => pick(role, (v as string) || null)}
                disabled={status === 'saving'}
              />
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
