<script lang="ts">
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import Switch from '$lib/components/Switch.svelte';

  const mcps = $derived(wizardState.instruments.mcps);
  const recommended = $derived(mcps.filter(m => m.recommended));
  const others = $derived(mcps.filter(m => !m.recommended));
  const installCount = $derived(mcps.filter(m => m.selected).length);

  // Derive the detected stack from the projects slice — every confirmed
  // project contributes its languages/frameworks/runtimes/services. The
  // chips are de-duplicated.
  const stackChips = $derived.by(() => {
    const seen = new Set<string>();
    for (const p of wizardState.projects.projects) {
      for (const tag of [
        ...(p.stack.languages ?? []),
        ...(p.stack.frameworks ?? []),
        ...(p.stack.runtimes ?? []),
        ...(p.stack.services ?? []),
      ]) {
        if (tag) seen.add(tag);
      }
    }
    return [...seen];
  });
</script>

<div class="max-w-[820px]">
  <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
    Tools sensei can reach for — recommended based on what's in your stack. Each
    MCP brings its own capabilities, no wrapping needed.
  </p>

  <!-- Detected stack -->
  {#if stackChips.length > 0}
    <div class="text-xs uppercase tracking-wider text-surface-z6 mb-2">
      Detected in your stack
    </div>
    <div data-testid="instruments-stack" class="flex flex-wrap gap-1 mb-6 bg-surface-z2 border border-surface-z3 rounded-md p-3">
      {#each stackChips as chip}
        <span class="mono py-1 px-2 text-xs text-surface-z7 bg-surface-z1 border border-surface-z3 rounded-sm">{chip}</span>
      {/each}
    </div>
  {/if}

  {#if mcps.length === 0}
    <div data-testid="instruments-empty" class="text-center p-12 bg-surface-z2 rounded-lg border border-surface-z3">
      <span class="kanji text-4xl text-primary-z5 opacity-20 block mb-4">器</span>
      <p class="text-sm text-surface-z6 m-0">
        No instruments available yet.
      </p>
      <p class="text-xs text-surface-z6 mt-2 m-0 mx-auto max-w-[420px]">
        The MCP registry is wired up but the daemon has no endpoint exposing
        recommendations yet. Once available, recommended MCPs will appear here
        based on the stack detected above.
      </p>
    </div>
  {:else}
    <div class="flex items-center gap-2 mb-6">
      <span class="mono py-1 px-2 text-xs text-success-z6 bg-surface-z2 border border-success-z5 rounded-sm">
        {installCount} MCPs to install
      </span>
    </div>

    {#if recommended.length > 0}
      <div class="text-xs uppercase tracking-wider text-surface-z6 mb-2">
        Recommended for your stack
      </div>
      <div class="flex flex-col gap-2 mb-6">
        {#each recommended as mcp (mcp.id)}
          <div
            data-testid={`mcp-card-${mcp.id}`}
            data-selected={mcp.selected}
            class="grid grid-cols-[auto_1fr_auto_auto] gap-3 px-4 py-3 bg-surface-z2 border border-surface-z3 rounded-md items-center transition-opacity duration-fast"
            class:opacity-55={!mcp.selected && !mcp.installed}
          >
            <span class="kanji text-xl text-primary-z6 w-9 h-9 flex items-center justify-center rounded-full bg-surface-z1 border border-surface-z3">
              器
            </span>
            <div class="min-w-0">
              <div class="flex items-baseline gap-2">
                <span class="text-sm text-surface-z9 font-medium">{mcp.name}</span>
                <span class="mono text-[11px] text-surface-z6">{mcp.publisher}</span>
                {#if mcp.verified}
                  <span class="text-[11px] text-success-z6" title="Verified">✓</span>
                {/if}
              </div>
              <div class="text-xs text-surface-z6 mt-0.5">{mcp.summary}</div>
            </div>
            <span class="mono text-[11px] text-surface-z6 whitespace-nowrap">{mcp.tools} tools</span>
            {#if mcp.installed}
              <span class="mono text-[11px] text-success-z6 whitespace-nowrap">installed</span>
            {:else}
              <Switch bind:value={mcp.selected} label={`Install ${mcp.name}`} />
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if others.length > 0}
      <div class="text-xs uppercase tracking-wider text-surface-z6 mb-2">
        Also available
      </div>
      <div class="flex flex-col gap-2">
        {#each others as mcp (mcp.id)}
          <div
            data-testid={`mcp-card-${mcp.id}`}
            data-selected={mcp.selected}
            class="grid grid-cols-[auto_1fr_auto_auto] gap-3 px-4 py-3 bg-surface-z2 border border-surface-z3 rounded-md items-center transition-opacity duration-fast"
            class:opacity-60={!mcp.selected && !mcp.installed}
          >
            <span class="kanji text-xl text-surface-z6 w-9 h-9 flex items-center justify-center rounded-full bg-surface-z1 border border-surface-z3">
              器
            </span>
            <div class="min-w-0">
              <div class="flex items-baseline gap-2">
                <span class="text-sm text-surface-z9 font-medium">{mcp.name}</span>
                <span class="mono text-[11px] text-surface-z6">{mcp.publisher}</span>
              </div>
              <div class="text-xs text-surface-z6 mt-0.5">{mcp.summary}</div>
            </div>
            <span class="mono text-[11px] text-surface-z6 whitespace-nowrap">{mcp.tools} tools</span>
            {#if mcp.installed}
              <span class="mono text-[11px] text-success-z6 whitespace-nowrap">installed</span>
            {:else}
              <Switch bind:value={mcp.selected} label={`Install ${mcp.name}`} />
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>
