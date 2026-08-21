<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import Switch from '$lib/components/Switch.svelte';

  let loading = $state(true);
  let error = $state<string | null>(null);

  const mcps = $derived(wizardState.instruments.mcps);
  const recommended = $derived(mcps.filter(m => m.recommended));
  const others = $derived(mcps.filter(m => !m.recommended));
  const installCount = $derived(mcps.filter(m => m.selected).length);

  onMount(async () => {
    try {
      await wizardState.refreshInstruments();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  // Derive the detected stack from the projects slice — every confirmed
  // project contributes its languages/frameworks/runtimes/services. The
  // chips are de-duplicated.
  const stackChips = $derived.by(() => {
    const seen = new Set<string>();
    for (const p of wizardState.projects.projects) {
      for (const tag of [
        ...(p.stack?.languages ?? []),
        ...(p.stack?.frameworks ?? []),
        ...(p.stack?.runtimes ?? []),
        ...(p.stack?.services ?? []),
      ]) {
        if (tag) seen.add(tag);
      }
    }
    return [...seen];
  });
</script>

<div class="max-w-[820px]">
  <p class="text-sm text-ink-soft leading-normal m-0 mb-6">
    Tools sensei can reach for — recommended based on what's in your stack. Each
    MCP brings its own capabilities, no wrapping needed.
  </p>

  <!-- Detected stack -->
  {#if stackChips.length > 0}
    <div class="text-xs uppercase tracking-wider text-ink-soft mb-2">
      Detected in your stack
    </div>
    <div data-testid="instruments-stack" class="flex flex-wrap gap-1 mb-6 bg-paper-mute border border-paper-edge rounded-md p-3">
      {#each stackChips as chip}
        <span class="font-mono py-1 px-2 text-xs text-ink-mute bg-paper-soft border border-paper-edge rounded-sm">{chip}</span>
      {/each}
    </div>
  {/if}

  {#if mcps.length === 0}
    <div data-testid="instruments-empty" class="text-center p-12 bg-paper-mute rounded-lg border border-paper-edge">
      <span class="kanji text-4xl text-accent opacity-20 block mb-4">器</span>
      <p class="text-sm text-ink-soft m-0">
        No instruments available yet.
      </p>
      <p class="text-xs text-ink-soft mt-2 m-0 mx-auto max-w-[420px]">
        The MCP registry is wired up but the daemon has no endpoint exposing
        recommendations yet. Once available, recommended MCPs will appear here
        based on the stack detected above.
      </p>
    </div>
  {:else}
    <div class="flex items-center gap-2 mb-6">
      <span class="font-mono py-1 px-2 text-xs text-success bg-success-soft border border-success rounded-sm">
        {installCount} MCPs to install
      </span>
    </div>

    {#if recommended.length > 0}
      <div class="text-xs uppercase tracking-wider text-ink-soft mb-2">
        Recommended for your stack
      </div>
      <div class="flex flex-col gap-2 mb-6">
        {#each recommended as mcp (mcp.id)}
          <div
            data-testid={`mcp-card-${mcp.id}`}
            data-selected={mcp.selected}
            class="grid grid-cols-[auto_1fr_auto_auto] gap-3 px-4 py-3 bg-paper-mute border border-paper-edge rounded-md items-center transition-opacity duration-fast"
            class:opacity-55={!mcp.selected && !mcp.installed}
          >
            <span class="kanji text-xl text-accent w-9 h-9 flex items-center justify-center rounded-full bg-paper-soft border border-paper-edge">
              器
            </span>
            <div class="min-w-0">
              <div class="flex items-baseline gap-2">
                <span class="text-sm text-ink font-medium">{mcp.name}</span>
                <span class="font-mono text-xs text-ink-soft">{mcp.publisher}</span>
                {#if mcp.verified}
                  <span class="text-xs text-success" title="Verified">✓</span>
                {/if}
              </div>
              <div class="text-xs text-ink-soft mt-0.5">{mcp.summary}</div>
            </div>
            <span class="font-mono text-xs text-ink-soft whitespace-nowrap">{mcp.tools} tools</span>
            {#if mcp.installed}
              <span class="font-mono text-xs text-success border border-success rounded-sm px-1.5 py-0.5 uppercase tracking-wide whitespace-nowrap">installed</span>
            {:else}
              <Switch bind:value={mcp.selected} label={`Install ${mcp.name}`} />
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if others.length > 0}
      <div class="text-xs uppercase tracking-wider text-ink-soft mb-2">
        Also available
      </div>
      <div class="flex flex-col gap-2">
        {#each others as mcp (mcp.id)}
          <div
            data-testid={`mcp-card-${mcp.id}`}
            data-selected={mcp.selected}
            class="grid grid-cols-[auto_1fr_auto_auto] gap-3 px-4 py-3 bg-paper-mute border border-paper-edge rounded-md items-center transition-opacity duration-fast"
            class:opacity-60={!mcp.selected && !mcp.installed}
          >
            <span class="kanji text-xl text-ink-soft w-9 h-9 flex items-center justify-center rounded-full bg-paper-soft border border-paper-edge">
              器
            </span>
            <div class="min-w-0">
              <div class="flex items-baseline gap-2">
                <span class="text-sm text-ink font-medium">{mcp.name}</span>
                <span class="font-mono text-xs text-ink-soft">{mcp.publisher}</span>
              </div>
              <div class="text-xs text-ink-soft mt-0.5">{mcp.summary}</div>
            </div>
            <span class="font-mono text-xs text-ink-soft whitespace-nowrap">{mcp.tools} tools</span>
            {#if mcp.installed}
              <span class="font-mono text-xs text-success border border-success rounded-sm px-1.5 py-0.5 uppercase tracking-wide whitespace-nowrap">installed</span>
            {:else}
              <Switch bind:value={mcp.selected} label={`Install ${mcp.name}`} />
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>
