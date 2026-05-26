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
  <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
    Libraries without their own MCP — sensei indexes docs &amp; code and wraps
    them with its own tools. Anything with a proper MCP (like Postgres or Stripe)
    comes in the next step.
  </p>

  {#if loading}
    <div data-testid="libraries-loading" class="text-center p-12 bg-surface-z2 rounded-lg border border-surface-z3">
      <span class="kanji text-4xl text-primary-z5 opacity-20 block mb-4">書</span>
      <p class="text-sm text-surface-z6">Loading libraries…</p>
    </div>
  {:else if error}
    <div data-testid="libraries-error" class="mb-6 p-4 rounded-md border border-danger-z5 bg-surface-z2">
      <div class="text-sm font-semibold text-danger-z5">Could not load libraries</div>
      <div class="text-xs text-surface-z7 mt-1 font-mono">{error}</div>
    </div>
  {:else if libs.length === 0}
    <div data-testid="libraries-empty" class="text-center p-12 bg-surface-z2 rounded-lg border border-surface-z3">
      <span class="kanji text-4xl text-primary-z5 opacity-20 block mb-4">書</span>
      <p class="text-sm text-surface-z6">No libraries detected yet. Run the scan stage first.</p>
    </div>
  {:else}
    <!-- Summary chips -->
    <div class="flex items-center gap-2 mb-6" data-testid="libraries-summary">
      <span class="mono py-1 px-2 text-xs text-surface-z7 bg-surface-z2 border border-surface-z3 rounded-sm">
        {detectedCount} detected
      </span>
      <span class="mono py-1 px-2 text-xs text-success-z6 bg-surface-z2 border border-success-z5 rounded-sm">
        {wrappedCount} will be wrapped
      </span>
    </div>

    <!-- Section label -->
    <div class="text-xs uppercase tracking-wider text-surface-z6 mb-2">
      Detected · sensei will wrap
    </div>

    <!-- Library list -->
    <div class="flex flex-col bg-surface-z2 border border-surface-z3 rounded-md overflow-hidden">
      {#each libs as lib, i (lib.id)}
        <div
          data-testid={`library-row-${lib.id}`}
          data-enabled={lib.enabled}
          class="grid grid-cols-[1fr_auto_auto] gap-3 py-3 px-4 items-center transition-opacity duration-fast"
          class:opacity-45={!lib.enabled}
          class:border-t={i > 0}
          class:border-surface-z3={i > 0}
        >
          <!-- Name + repos -->
          <div class="min-w-0">
            <div class="flex items-baseline gap-2">
              <span class="text-sm text-surface-z9 font-medium truncate">{lib.name}</span>
              {#if lib.version}
                <span class="mono text-[11px] text-surface-z6 bg-surface-z3 rounded-sm px-1.5 py-0.5">{lib.version}</span>
              {/if}
              {#if lib.ecosystem}
                <span class="mono text-[11px] text-surface-z6 uppercase">{lib.ecosystem}</span>
              {/if}
            </div>
            {#if lib.description}
              <div class="text-xs text-surface-z6 mt-0.5 truncate">{lib.description}</div>
            {:else if lib.repos.length > 0}
              <div class="mono text-xs text-surface-z6 mt-0.5 truncate">
                used by {lib.repos.slice(0, 3).join(', ')}{lib.repos.length > 3 ? ` +${lib.repos.length - 3} more` : ''}
              </div>
            {/if}
          </div>

          <!-- Usage count -->
          <span class="mono text-xs text-surface-z6 whitespace-nowrap">
            {lib.repoCount} repo{lib.repoCount === 1 ? '' : 's'}
          </span>

          <!-- Toggle -->
          <Switch
            bind:value={lib.enabled}
            label={`Wrap ${lib.name}`}
          />
        </div>
      {/each}
    </div>
  {/if}
</div>
