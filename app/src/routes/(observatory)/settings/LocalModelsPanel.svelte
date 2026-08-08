<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { LocalModels, phaseDisplay, phaseError, type PhaseTone } from './local-models.svelte.js';

  // The controller is injectable so tests drive the panel with fixture phases
  // (a real pull is blocked upstream). Default: a live controller over the
  // daemon api, loaded on mount.
  let { controller = new LocalModels(senseiApi(appState.port)) }: { controller?: LocalModels } = $props();

  // Kick off the initial load (and any resulting poll) once, on the client.
  // A fixture-backed test controller pre-seeds `models`; calling `load()` on it
  // is harmless (its mock api resolves the same fixtures).
  onMount(() => { void controller.load(); });
  onDestroy(() => controller.dispose());

  // Named-token text class per tone — kept here so the template stays literal
  // and UnoCSS statically sees the classes.
  const toneClass: Record<PhaseTone, string> = {
    'ink-mute': 'text-ink-mute',
    'ink-soft': 'text-ink-soft',
    success: 'text-success',
    warning: 'text-warning',
    accent: 'text-accent',
  };
</script>

<div class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg" data-testid="settings-local-models">
  <h3 class="text-base m-0 mb-1">Local models</h3>
  <p class="text-sm text-ink-soft m-0 mb-5">
    Pulled to disk, served in-process by the embedded runtime — runs offline.
  </p>

  {#if controller.notice}
    <p class="text-sm text-ink-soft" data-testid="local-models-notice">{controller.notice}</p>
  {:else if controller.loading}
    <p class="text-sm text-ink-soft">Loading models…</p>
  {:else if controller.models.length === 0}
    <p class="text-sm text-ink-soft" data-testid="local-models-empty">No local models configured.</p>
  {:else}
    {#if controller.error}
      <p class="text-xs text-danger mb-3" data-testid="local-models-error">{controller.error}</p>
    {/if}
    <div class="flex flex-col divide-y divide-paper-edge">
      {#each controller.models as m (m.id)}
        {@const d = phaseDisplay(m.phase)}
        {@const err = phaseError(m.phase)}
        <div class="grid grid-cols-[1fr_auto] gap-3 py-3 items-center" data-testid={`local-model-${m.id}`}>
          <div class="min-w-0">
            <div class="font-mono text-sm text-ink truncate">{m.id}</div>
            <div class="text-xs text-ink-soft mt-0.5">{m.name}</div>
            {#if d.percent !== null}
              <div class="h-1 mt-2 rounded-full bg-paper-edge overflow-hidden" data-testid={`local-model-progress-${m.id}`}>
                <div class="h-full bg-accent" style="width: {d.percent}%"></div>
              </div>
            {/if}
          </div>
          <div class="flex items-center gap-3 justify-end">
            {#if d.actionable}
              <button
                type="button"
                class="text-xs px-2.5 py-1.5 border border-paper-edge rounded-md bg-paper-soft text-ink cursor-pointer"
                data-testid={`local-model-pull-${m.id}`}
                title={err || undefined}
                onclick={() => controller.pull(m.id)}
              >{m.phase.phase === 'failed' ? 'Retry' : 'Pull'}</button>
            {:else}
              <span
                class="text-xs {toneClass[d.tone]}"
                data-testid={`local-model-status-${m.id}`}
                title={err || undefined}
              >{d.label}</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
