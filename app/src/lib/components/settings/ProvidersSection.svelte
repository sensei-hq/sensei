<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

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

  /** Per-row inline save. Settings has no Continue mechanic, so each row
   *  persists on demand via the Save button — same daemon endpoint the
   *  wizard's commit-hook used. Runs on the row's `router` object so the
   *  status chips (`saveState`) reflect progress. */
  async function saveKey(router: { id: string; draftKey: string; saveState: string; saveError: string; configured: boolean }): Promise<void> {
    const trimmed = router.draftKey.trim();
    if (trimmed.length === 0) return;
    router.saveState = 'saving';
    router.saveError = '';
    const result = await senseiApi(appState.port).setGatewayRouterKey(router.id, trimmed);
    if (result.ok) {
      router.configured = result.data.configured;
      router.draftKey = '';
      router.saveState = 'done';
    } else {
      router.saveState = 'failed';
      router.saveError = result.error.message;
    }
  }
</script>

<div class="max-w-[820px]">
  <p class="text-sm text-ink-soft leading-normal m-0 mb-6">
    Connect inference providers so sensei can route chat, embedding, and
    image-generation requests on your behalf. Keys are stored in the macOS
    Keychain — never on disk in plaintext, never in the database.
  </p>

  {#if loading}
    <div data-testid="inference-loading" class="text-center p-12 bg-paper-mute rounded-lg border border-paper-edge">
      <span class="kanji text-4xl text-accent opacity-20 block mb-4">想</span>
      <p class="text-sm text-ink-soft">Loading routers…</p>
    </div>
  {:else if error}
    <div data-testid="inference-error" class="mb-6 p-4 rounded-md border border-danger bg-paper-mute">
      <div class="text-sm font-semibold text-danger">Could not load routers</div>
      <div class="text-xs text-ink-mute mt-1 font-mono">{error}</div>
    </div>
  {:else}
    <div class="flex items-center gap-2 mb-6">
      <span class="font-mono py-1 px-2 text-xs text-success bg-success-soft border border-success rounded-sm">
        {configuredCount} of {routers.length} configured
      </span>
    </div>

    <div class="flex flex-col gap-3">
      {#each routers as router (router.id)}
        <div
          data-testid={`router-card-${router.id}`}
          data-configured={router.configured}
          data-save-state={router.saveState}
          class="bg-paper-mute border border-paper-edge rounded-md p-4 flex flex-col gap-3"
        >
          <div class="flex items-baseline justify-between gap-3">
            <div class="flex items-baseline gap-2 min-w-0">
              <span class="text-sm text-ink font-semibold">{router.name}</span>
              <span class="font-mono text-[11px] text-ink-soft uppercase">{router.id}</span>
            </div>
            {#if router.saveState === 'saving'}
              <span class="text-xs text-accent font-mono">saving…</span>
            {:else if router.saveState === 'done'}
              <span class="text-xs text-success font-mono">saved ✓</span>
            {:else if router.saveState === 'failed'}
              <span class="text-xs text-danger font-mono">failed</span>
            {:else if router.configured}
              <span class="text-xs text-success font-mono">configured ✓</span>
            {:else if !router.needsKey}
              <span class="text-xs text-ink-soft font-mono">no key needed</span>
            {:else}
              <span class="text-xs text-ink-soft font-mono">not configured</span>
            {/if}
          </div>

          <!-- Providers + capabilities chips -->
          <div class="flex flex-wrap gap-1">
            {#each router.providers as p}
              <span class="font-mono text-[11px] text-ink-mute bg-paper-soft border border-paper-edge rounded-sm px-1.5 py-0.5">{p}</span>
            {/each}
            {#each router.capabilities as c}
              <span class="font-mono text-[11px] text-accent bg-paper-soft border border-accent rounded-sm px-1.5 py-0.5">{c}</span>
            {/each}
          </div>

          {#if router.needsKey}
            <div class="flex items-center gap-2">
              <input
                type="password"
                bind:value={router.draftKey}
                placeholder={router.configured ? 'Update key (paste to replace)' : 'Paste API key (sk-...)'}
                data-testid={`router-key-input-${router.id}`}
                onkeydown={(e) => { if (e.key === 'Enter') void saveKey(router); }}
                class="key-input flex-1 px-3 py-1.5 text-sm font-mono bg-paper-soft border border-paper-edge rounded-md text-ink focus:outline-none focus:border-accent"
              />
              <button
                type="button"
                data-testid={`router-save-${router.id}`}
                class="btn-solid text-xs"
                disabled={router.draftKey.trim().length === 0 || router.saveState === 'saving'}
                onclick={() => saveKey(router)}
              >Save</button>
              {#if router.configured}
                <button
                  type="button"
                  data-testid={`router-clear-${router.id}`}
                  class="text-xs text-ink-soft hover:text-danger cursor-pointer bg-none border-none"
                  onclick={() => wizardState.clearInferenceRouterKey(router.id)}
                >Clear</button>
              {/if}
            </div>
            {#if router.saveError}
              <p class="text-xs text-danger font-mono m-0">{router.saveError}</p>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .key-input::placeholder {
    color: var(--ink-soft);
  }
  .key-input:focus {
    border-color: var(--accent);
  }
</style>
