<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import Switch from '$lib/components/Switch.svelte';

  let loading = $state(true);
  let error = $state<string | null>(null);

  const libs = $derived(wizardState.libraries.libs);
  const detectedCount = $derived(libs.length);
  const wrappedCount = $derived(libs.filter(l => l.enabled).length);

  onMount(async () => {
    try {
      await wizardState.refreshLibraries();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="max-w-[820px]">
  <p class="text-sm text-ink-soft leading-normal m-0 mb-6">
    Libraries without their own MCP — sensei indexes docs &amp; code and wraps
    them with its own tools. Anything with a proper MCP (like Postgres or Stripe)
    is managed under Instruments.
  </p>

  {#if loading}
    <div data-testid="libraries-loading" class="text-center p-12 bg-paper-mute rounded-lg border border-paper-mute">
      <span class="kanji text-4xl text-accent opacity-20 block mb-4">書</span>
      <p class="text-sm text-ink-soft">Loading libraries…</p>
    </div>
  {:else if error}
    <div data-testid="libraries-error" class="mb-6 p-4 rounded-md border border-danger bg-paper-mute">
      <div class="text-sm font-semibold text-danger">Could not load libraries</div>
      <div class="text-xs text-ink-mute mt-1 font-mono select-text">{error}</div>
    </div>
  {:else if libs.length === 0}
    <div data-testid="libraries-empty" class="text-center p-12 bg-paper-mute rounded-lg border border-paper-mute">
      <span class="kanji text-4xl text-accent opacity-20 block mb-4">書</span>
      <p class="text-sm text-ink-soft">No libraries detected yet. Run the scan stage first.</p>
    </div>
  {:else}
    <div class="flex items-center gap-2 mb-6" data-testid="libraries-summary">
      <span class="font-mono py-1 px-2 text-xs text-ink-mute bg-paper-mute border border-paper-mute rounded-sm">
        {detectedCount} detected
      </span>
      <span class="font-mono py-1 px-2 text-xs text-success bg-success-soft border border-success rounded-sm">
        {wrappedCount} will be wrapped
      </span>
    </div>

    <div class="text-xs uppercase tracking-wider text-ink-soft mb-2">
      Detected · sensei will wrap
    </div>

    <div class="flex flex-col bg-paper-mute border border-paper-mute rounded-md overflow-hidden">
      {#each libs as lib, i (lib.id)}
        <div
          data-testid={`library-row-${lib.id}`}
          data-enabled={lib.enabled}
          class="grid grid-cols-[1fr_auto_auto] gap-3 py-3 px-4 items-center transition-opacity duration-fast"
          class:opacity-45={!lib.enabled}
          class:border-t={i > 0}
          class:border-paper-mute={i > 0}
        >
          <div class="min-w-0">
            <div class="flex items-baseline gap-2">
              <span class="text-sm text-ink font-medium truncate">{lib.name}</span>
              {#if lib.version}
                <span class="font-mono text-[11px] text-ink-soft bg-paper-mute rounded-sm px-1.5 py-0.5">{lib.version}</span>
              {/if}
              {#if lib.ecosystem}
                <span class="font-mono text-[11px] text-ink-soft uppercase">{lib.ecosystem}</span>
              {/if}
            </div>
            {#if lib.description}
              <div class="text-xs text-ink-soft mt-0.5 truncate">{lib.description}</div>
            {:else if lib.repos.length > 0}
              <div class="font-mono text-xs text-ink-soft mt-0.5 truncate">
                used by {lib.repos.slice(0, 3).join(', ')}{lib.repos.length > 3 ? ` +${lib.repos.length - 3} more` : ''}
              </div>
            {/if}
          </div>

          <span class="font-mono text-xs text-ink-soft whitespace-nowrap">
            {lib.repoCount} repo{lib.repoCount === 1 ? '' : 's'}
          </span>

          <Switch
            bind:value={lib.enabled}
            label={`Wrap ${lib.name}`}
          />
        </div>
      {/each}
    </div>
  {/if}
</div>
