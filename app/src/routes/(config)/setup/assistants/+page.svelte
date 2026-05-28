<script lang="ts">
  import { wizardState, familyIsConfigured } from '$lib/wizard-state.svelte.js';
  import Switch from '$lib/components/Switch.svelte';

  const assistants = $derived(wizardState.assistants.assistants);
  const configState = $derived(wizardState.assistants.configureState);
  const configError = $derived(wizardState.assistants.configureError);

  // What gets registered when each family is enabled.
  // Claude gets the full plugin suite; all others get MCP server only.
  const CAPABILITIES: Record<string, string[]> = {
    claude: ['plugins', 'skills', 'commands', 'agents'],
  };

  function caps(id: string): string[] {
    return CAPABILITIES[id] ?? ['mcp server'];
  }
</script>

<div>
  <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
    Registers plugins, skills, commands, agents, logging and metrics.
  </p>

  <div class="grid grid-cols-2 gap-3">
    {#each assistants as fam (fam.id)}
      {@const installedCount = fam.variants.filter(v => v.installed).length}
      {@const anyInstalled = installedCount > 0}
      {@const isConfigured = familyIsConfigured(fam)}
      {@const state = configState[fam.id] ?? 'idle'}
      {@const err = configError[fam.id]}
      <div
        data-testid={`assistant-card-${fam.id}`}
        data-configure-state={state}
        data-configured={isConfigured}
        class="card flex items-center gap-4 px-6 py-5 rounded-lg bg-surface-z1 border border-surface-z3 transition-all duration-fast min-w-0"
        class:card-selected={fam.selected}
        class:card-missing={!anyInstalled}
      >
        <div class="flex-1 min-w-0">
          <div class="flex items-baseline justify-between gap-2 mb-1.5">
            <span class="text-base font-semibold">{fam.name}</span>
            {#if state === 'configuring'}
              <span class="text-xs text-primary-z6 whitespace-nowrap mono">configuring…</span>
            {:else if state === 'removing'}
              <span class="text-xs text-warning-z6 whitespace-nowrap mono">removing…</span>
            {:else if state === 'failed'}
              <span class="text-xs text-danger-z5 whitespace-nowrap mono">failed</span>
            {:else if state === 'skipped'}
              <span class="text-xs text-surface-z5 whitespace-nowrap mono">skipped</span>
            {:else if isConfigured}
              <span class="text-xs text-success-z6 whitespace-nowrap mono">configured ✓</span>
            {:else if anyInstalled}
              <span class="text-xs text-surface-z5 whitespace-nowrap">{installedCount} detected</span>
            {:else}
              <span class="text-xs text-surface-z5 italic whitespace-nowrap">not found</span>
            {/if}
          </div>
          <div class="flex flex-wrap gap-1">
            {#each caps(fam.id) as cap}
              <span class="chip text-xs font-mono text-surface-z7 px-2 py-0.5 bg-surface-z3 rounded-sm whitespace-nowrap">{cap}</span>
            {/each}
          </div>
          {#if err}
            <p class="text-xs text-danger-z5 font-mono mt-2 m-0 break-words">{err}</p>
          {/if}
        </div>
        <Switch
          bind:value={fam.selected}
          label={`Enable ${fam.name}`}
        />
      </div>
    {/each}
  </div>

  {#if assistants.length === 0}
    <p class="text-sm text-surface-z6 italic">
      No AI coding assistants detected. Make sure the daemon is running.
    </p>
  {/if}
</div>

<style>
  .card-selected {
    border: 2px solid oklch(var(--color-primary-z5) / 1);
    background: oklch(var(--color-surface-z2) / 1);
  }
  .card-selected .chip {
    background: oklch(var(--color-surface-z1) / 1);
  }
  .card-missing {
    opacity: 0.55;
  }
</style>
