<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
  import { onMount } from 'svelte';
  import { wizardState, routerKeyStatus, routerSaveNotice, saveRouterKey, type RouterEntry } from '$lib/wizard-state.svelte.js';
  import { Notice } from '@sensei/ui';
  import { Button } from '@rokkit/ui';
  import { SectionRead } from '$lib/section-read.svelte.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  const read = new SectionRead("Couldn't load routers");

  const routers = $derived(wizardState.inference.routers);
  const configuredCount = $derived(routers.filter(r => r.configured).length);

  onMount(() => read.run(() => wizardState.refreshInferenceRouters()));

  /** Per-row inline save — the wizard's own key save (saveRouterKey); Settings
   *  has no Continue step, so each row saves on demand. */
  const saveKey = (router: RouterEntry) => saveRouterKey(router, senseiApi(appState.port));
</script>

<div class="max-w-[820px]">
  <p class="text-sm text-ink-mute leading-normal m-0 mb-6">
    Connect inference providers so sensei can route chat, embedding, and
    image-generation requests on your behalf. Keys are stored in the macOS
    Keychain — never on disk in plaintext, never in the database.
  </p>

  {#if read.loading}
    <div data-testid="inference-loading" class="text-center p-12 bg-paper-mute rounded-lg border border-paper-edge">
      <Kanji char="想" size="inherit" tone="inherit" class="text-4xl text-accent opacity-20 block mb-4" />
      <p class="text-sm text-ink-mute">Loading routers…</p>
    </div>
  {:else if read.notice}
    <div class="mb-6"><Notice size="panel" {...read.notice} data-testid="inference-error" /></div>
  {:else}
    <div class="flex items-center gap-2 mb-6">
      <span class="font-mono py-1 px-2 text-xs text-on-success-soft bg-success-soft border border-success rounded-sm">
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
              <span class="font-mono text-xs text-ink-mute uppercase">{router.id}</span>
            </div>
            {#if router.saveState === 'saving'}
              <span class="text-xs text-accent font-mono">saving…</span>
            {:else if router.saveState === 'done'}
              <span class="text-xs text-success font-mono">saved ✓</span>
            {:else if router.saveState === 'failed'}
              <span class="text-xs text-ink-mute font-mono">not saved</span>
            {:else}
              {@const status = routerKeyStatus(router)}
              <span
                class="text-xs font-mono {status.tone === 'success' ? 'text-success' : status.tone === 'warning' ? 'text-warning' : 'text-ink-mute'}"
                data-key-status={status.tone}
              >{status.label}</span>
            {/if}
          </div>

          <!-- Providers + capabilities chips -->
          <div class="flex flex-wrap gap-1">
            {#each router.providers as p}
              <span class="font-mono text-xs text-ink-soft bg-paper-soft border border-paper-edge rounded-sm px-1.5 py-0.5">{p}</span>
            {/each}
            {#each router.capabilities as c}
              <span class="font-mono text-xs text-accent bg-paper-soft border border-accent rounded-sm px-1.5 py-0.5">{c}</span>
            {/each}
          </div>

          {#if router.needsKey}
            <div class="flex items-center gap-2">
              <input
                type="password"
                bind:value={router.draftKey}
                placeholder={router.configured !== false ? 'Update key (paste to replace)' : 'Paste API key (sk-...)'}
                data-testid={`router-key-input-${router.id}`}
                onkeydown={(e) => { if (e.key === 'Enter') void saveKey(router); }}
                class="key-input flex-1 px-3 py-1.5 text-sm font-mono bg-paper-soft border border-paper-edge rounded-md text-ink focus:outline-none focus:border-accent"
              />
              <Button
                variant="primary"
                size="sm"
                data-testid={`router-save-${router.id}`}
                disabled={router.draftKey.trim().length === 0 || router.saveState === 'saving'}
                onclick={() => saveKey(router)}
                label="Save"
              />
              {#if router.configured}
                <Button
                  variant="danger"
                  style="ghost"
                  size="sm"
                  data-testid={`router-clear-${router.id}`}
                  onclick={() => wizardState.clearInferenceRouterKey(router.id)}
                  label="Clear"
                />
              {/if}
            </div>
            {#if routerSaveNotice(router)}
              <Notice size="inline" bare expanded {...routerSaveNotice(router)!} />
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .key-input::placeholder {
    color: var(--ink-mute);
  }
  .key-input:focus {
    border-color: var(--accent);
  }
</style>
