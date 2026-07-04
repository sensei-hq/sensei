<script lang="ts">
  import { onMount } from 'svelte';
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

  onMount(loadChains);
</script>

<div class="px-7 py-7 bg-paper-mute border border-paper-mute rounded-lg" data-testid="settings-inference">
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
              <!-- Chain preview: model[1] → model[2] → …, primary marked -->
              <div class="flex items-center gap-2 mt-2 flex-wrap" title={current.models.map(m => m.modelName).join(' → ')}>
                {#each current.models as m, i (m.sequenceOrder)}
                  <span
                    class="font-mono text-xs px-2 py-0.5 rounded"
                    class:bg-ink={i === 0}
                    class:text-paper={i === 0}
                    class:bg-paper-mute={i > 0}
                    class:text-ink-mute={i > 0}
                  >{m.modelName}</span>
                  {#if i < current.models.length - 1}
                    <span class="text-xs text-ink-faint">→</span>
                  {/if}
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
            <select
              class="text-xs px-2.5 py-1.5 border border-paper-edge rounded-md bg-paper-soft text-ink cursor-pointer font-mono min-w-[160px]"
              data-testid={`inference-role-picker-${role}`}
              value={currentId ?? ''}
              onchange={(e) => pick(role, e.currentTarget.value || null)}
              disabled={status === 'saving'}
            >
              <option value="">— none —</option>
              {#each options as opt (opt.id)}
                <option value={opt.id}>{opt.name}</option>
              {/each}
            </select>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
