<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';

  let loading = $state(true);
  let error = $state<string | null>(null);

  const routers = $derived(wizardState.inference.routers);
  const configuredCount = $derived(routers.filter(r => r.configured).length);

  onMount(async () => {
    try {
      await wizardState.refreshInferenceRouters();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="max-w-[820px]">
  <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
    Connect inference providers so sensei can route chat, embedding, and
    image-generation requests on your behalf. Keys are stored in the macOS
    Keychain — never on disk in plaintext, never in the database.
  </p>

  {#if loading}
    <div data-testid="inference-loading" class="text-center p-12 bg-surface-z2 rounded-lg border border-surface-z3">
      <span class="kanji text-4xl text-primary-z5 opacity-20 block mb-4">想</span>
      <p class="text-sm text-surface-z6">Loading routers…</p>
    </div>
  {:else if error}
    <div data-testid="inference-error" class="mb-6 p-4 rounded-md border border-danger-z5 bg-surface-z2">
      <div class="text-sm font-semibold text-danger-z5">Could not load routers</div>
      <div class="text-xs text-surface-z7 mt-1 font-mono">{error}</div>
    </div>
  {:else}
    <div class="flex items-center gap-2 mb-6">
      <span class="mono py-1 px-2 text-xs text-success-z6 bg-surface-z2 border border-success-z5 rounded-sm">
        {configuredCount} of {routers.length} configured
      </span>
    </div>

    <div class="flex flex-col gap-3">
      {#each routers as router (router.id)}
        <div
          data-testid={`router-card-${router.id}`}
          data-configured={router.configured}
          data-save-state={router.saveState}
          class="bg-surface-z2 border border-surface-z3 rounded-md p-4 flex flex-col gap-3"
        >
          <div class="flex items-baseline justify-between gap-3">
            <div class="flex items-baseline gap-2 min-w-0">
              <span class="text-sm text-surface-z9 font-semibold">{router.name}</span>
              <span class="mono text-[11px] text-surface-z6 uppercase">{router.id}</span>
            </div>
            {#if router.saveState === 'saving'}
              <span class="text-xs text-primary-z6 mono">saving…</span>
            {:else if router.saveState === 'done'}
              <span class="text-xs text-success-z6 mono">saved ✓</span>
            {:else if router.saveState === 'failed'}
              <span class="text-xs text-danger-z5 mono">failed</span>
            {:else if router.configured}
              <span class="text-xs text-success-z6 mono">configured ✓</span>
            {:else if !router.needsKey}
              <span class="text-xs text-surface-z6 mono">no key needed</span>
            {:else}
              <span class="text-xs text-surface-z5 mono">not configured</span>
            {/if}
          </div>

          <!-- Providers + capabilities chips -->
          <div class="flex flex-wrap gap-1">
            {#each router.providers as p}
              <span class="mono text-[11px] text-surface-z7 bg-surface-z1 border border-surface-z3 rounded-sm px-1.5 py-0.5">{p}</span>
            {/each}
            {#each router.capabilities as c}
              <span class="mono text-[11px] text-primary-z6 bg-surface-z1 border border-primary-z2 rounded-sm px-1.5 py-0.5">{c}</span>
            {/each}
          </div>

          {#if router.needsKey}
            <div class="flex items-center gap-2">
              <input
                type="password"
                bind:value={router.draftKey}
                placeholder={router.configured ? 'Update key (paste to replace)' : 'Paste API key (sk-...)'}
                data-testid={`router-key-input-${router.id}`}
                class="key-input flex-1 px-3 py-1.5 text-sm font-mono bg-surface-z1 border border-surface-z3 rounded-md text-surface-z9 focus:outline-none focus:border-primary-z5"
              />
              {#if router.configured}
                <button
                  type="button"
                  data-testid={`router-clear-${router.id}`}
                  class="text-xs text-surface-z6 hover:text-danger-z5 cursor-pointer bg-none border-none"
                  onclick={() => wizardState.clearInferenceRouterKey(router.id)}
                >Clear</button>
              {/if}
            </div>
            {#if router.saveError}
              <p class="text-xs text-danger-z5 font-mono m-0">{router.saveError}</p>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .key-input::placeholder {
    color: oklch(var(--color-surface-z5) / 1);
  }
  .key-input:focus {
    border-color: oklch(var(--color-primary-z5) / 1);
  }
</style>
