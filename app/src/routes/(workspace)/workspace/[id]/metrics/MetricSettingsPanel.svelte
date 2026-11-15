<script lang="ts">
  // The SDLC settings panel (docs/spec/screen/workspace-metrics-settings.md).
  // PRESENTATIONAL: values, words and requests are metric-settings-state's.
  import { SETTINGS, type MetricSettingsState } from './metric-settings-state.svelte.js';

  let { panel }: { panel: MetricSettingsState } = $props();

  const button = 'rounded-md border border-paper-edge px-3 py-1.5 text-sm text-ink hover:bg-paper-mute disabled:opacity-50';
</script>

<section data-section="metric-settings" aria-labelledby="metric-settings-title" class="flex flex-col gap-3 rounded-lg border border-paper-edge bg-paper-soft p-4">
  <h2 id="metric-settings-title" class="m-0 font-heading text-base text-ink">SDLC settings</h2>
  {#if panel.view === 'loading'}
    <p class="m-0 text-sm text-ink-soft">Reading settings…</p>
  {:else if panel.view === 'error'}
    <p role="alert" class="m-0 text-sm text-danger">{panel.loadError}</p>
    <div><button type="button" class={button} data-action="retry" onclick={() => panel.load()}>Retry</button></div>
  {:else}
    <form
      class="flex flex-col gap-3"
      onsubmit={(e) => {
        e.preventDefault();
        void panel.save();
      }}
    >
      {#each SETTINGS as s (s.key)}
        {@const marked = panel.fieldError === s.key}
        <div class="flex flex-col gap-1">
          <label for="setting-{s.key}" class="text-sm text-ink">{s.label}</label>
          <input
            id="setting-{s.key}"
            data-setting={s.key}
            type="text"
            inputmode="decimal"
            class="max-w-[12rem] rounded-md border border-paper-edge bg-paper px-2 py-1 text-sm text-ink"
            aria-invalid={marked}
            aria-describedby="setting-{s.key}-help{marked ? ` setting-${s.key}-error` : ''}"
            bind:value={panel.draft[s.key]}
          />
          <p id="setting-{s.key}-help" class="m-0 text-xs text-ink-soft">{s.help}</p>
          {#if marked}
            <p id="setting-{s.key}-error" data-setting-error={s.key} class="m-0 text-xs text-danger">Check this value.</p>
          {/if}
        </div>
      {/each}
      {#if panel.alert}
        <p role="alert" class="m-0 text-sm text-danger">{panel.alert}</p>
      {/if}
      <div class="flex items-center gap-3">
        <button type="submit" class={button} data-action="save" disabled={!panel.dirty || panel.saving}>Save</button>
        {#if panel.notice}<span role="status" class="text-sm text-ink-mute">{panel.notice}</span>{/if}
      </div>
    </form>
  {/if}
</section>
