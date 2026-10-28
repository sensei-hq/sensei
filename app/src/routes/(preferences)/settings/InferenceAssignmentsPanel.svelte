<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
  import { onMount } from 'svelte';
  import { Select } from '@rokkit/ui';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { SENSEI_ROLES } from '$lib/setup/contracts.js';
  import { ROLE_META } from '$lib/setup/role-meta.js';
  import { ChainRoles } from '$lib/setup/chain-roles.svelte.js';

  // Loading, the role picks, member order and every save status are ChainRoles'
  // (audit 2026-10-28, item 13); this panel renders them.
  const roles = new ChainRoles(() => senseiApi(appState.port));

  onMount(() => roles.load());
</script>

<div class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg" data-testid="settings-inference">
  <h3 class="text-base m-0 mb-1">Inference</h3>
  <p class="text-sm text-ink-mute m-0 mb-5">
    Which model chain handles each reasoning role. Chains carry an ordered
    fallback list; change them here to try a different mix.
  </p>

  {#if roles.loading}
    <p class="text-sm text-ink-mute">Loading chains…</p>
  {:else if roles.loadError}
    <p class="text-sm text-danger" data-testid="settings-inference-error">Could not load chains: {roles.loadError}</p>
  {:else}
    <div class="flex flex-col divide-y divide-paper-edge">
      {#each SENSEI_ROLES as role (role)}
        {@const meta = ROLE_META[role]}
        {@const currentId = roles.assignments[role]}
        {@const current = roles.chainById(currentId)}
        {@const options = roles.optionsFor(role)}
        {@const status = roles.saveStatus[role]}
        <div class="grid grid-cols-[28px_1fr_auto] gap-3 py-4 items-start" data-testid={`inference-role-${role}`}>
          <Kanji char={meta.kanji} size="inherit" tone="inherit" class="text-xl text-accent leading-none mt-1" />
          <div class="min-w-0">
            <div class="text-sm text-ink font-medium">{meta.label}</div>
            <div class="text-xs text-ink-mute mt-0.5">{meta.hint}</div>
            {#if current && current.models.length > 0}
              <!-- Chain editor: model chips + ▲/▼/× per member. Adding a
                   new model happens in the wizard's fuller "Available
                   models" picker; here we keep the chain-model editor
                   compact — you can only reorder or remove. -->
              <div class="flex flex-col gap-1 mt-2" data-testid={`inference-chain-members-${role}`}>
                {#each current.models as m, i (m.memberId)}
                  {@const primary = i === 0}
                  {@const busy = roles.memberBusy[m.memberId]}
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
                      onclick={() => roles.moveMember(role, m.memberId, -1)}
                    >▲</button>
                    <button
                      type="button"
                      class="w-5 h-5 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={i === current.models.length - 1 || busy}
                      disabled={i === current.models.length - 1 || !!busy}
                      title="Move down"
                      data-testid={`inference-member-down-${m.memberId}`}
                      onclick={() => roles.moveMember(role, m.memberId, 1)}
                    >▼</button>
                    <button
                      type="button"
                      class="w-5 h-5 text-xs bg-transparent border-none cursor-pointer"
                      class:opacity-25={busy}
                      disabled={!!busy}
                      title="Remove"
                      data-testid={`inference-member-remove-${m.memberId}`}
                      onclick={() => roles.removeMember(role, m.memberId)}
                    >×</button>
                  </div>
                {/each}
              </div>
            {/if}
            {#if status === 'error' && roles.saveError[role]}
              <div class="text-xs text-danger mt-1" data-testid={`inference-role-error-${role}`}>
                {roles.saveError[role]}
              </div>
            {/if}
          </div>
          <div class="flex items-center gap-2">
            <span
              class="text-xs w-14 text-right"
              class:text-ink-soft={status === 'idle'}
              class:text-ink-mute={status === 'saving'}
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
                onchange={(v) => roles.pick(role, (v as string) || null)}
                disabled={status === 'saving'}
              />
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
