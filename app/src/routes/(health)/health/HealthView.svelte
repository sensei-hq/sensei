<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import Header from './Header.svelte';
  import Hero   from './Hero.svelte';
  import Remedy from './Remedy.svelte';
  import Ledger from './Ledger.svelte';

  interface Props {
    state: HealthState;
    onEnter?: () => void;
    onRecheck?: () => void;
    onCopyScript?: () => void;
  }
  let { state, onEnter, onRecheck, onCopyScript }: Props = $props();
</script>

<div class="flex-1 min-h-0 overflow-y-auto px-10 py-12">
  <div class="max-w-[640px] w-full mx-auto">
    <Header platform={state.platform} status={state.status} />
    <Hero
      packageManager={state.packageManager}
      status={state.status}
      components={state.components}
      {onEnter}
    />
    {#if state.needsAction && state.remedy}
      <Remedy remedy={state.remedy} {onCopyScript} {onRecheck} />
    {/if}
    <Ledger components={state.components} />
    <footer class="flex justify-between items-center gap-4 mt-8 pt-5.5 border-t border-surface-z2">
      <span class="text-2xs text-ink-z5">Bootstrap runs once. The next launch will be quick.</span>
      {#if state.isOk}
        <button data-action="continue" class="btn-solid" onclick={onEnter}>Continue →</button>
      {/if}
    </footer>
  </div>
</div>
