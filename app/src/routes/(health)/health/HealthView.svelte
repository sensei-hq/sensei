<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import Header from './Header.svelte';
  import Hero   from './Hero.svelte';
  import Remedy from './Remedy.svelte';
  import Ledger from './Ledger.svelte';
  import Footer from './Footer.svelte';

  interface Props {
    state: HealthState;
    onEnter?: () => void;
    onVerify?: () => void;
  }
  let { state, onEnter, onVerify }: Props = $props();

  // Ledger and Hero both consume the same 6-row array (PM + 5 components).
  // Building it once here keeps the two children visually in sync.
  const gates = $derived([state.packageManager, ...state.components]);
</script>

<div class="flex-1 min-h-0 overflow-y-auto px-8 py-10">
  <div class="max-w-[960px] w-full mx-auto grid grid-cols-1 lg:grid-cols-2 gap-x-12 gap-y-8 min-h-full">
    <!-- Left column · identity, headline, remedy, footer -->
    <div class="flex flex-col min-w-0">
      <Header status={state.status} />

      {#if state.needsAction && state.remedy}
        <Remedy remedy={state.remedy} {onVerify} />
      {/if}

      <div class="mt-auto pt-8">
        <Footer version={state.version} platform={state.platform} />
      </div>
    </div>

    <!-- Right column · hero status, ledger, continue -->
    <div class="flex flex-col gap-5 min-w-0">
      <Hero status={state.status} components={gates} />

      <div class="flex-1 min-h-0">
        <Ledger components={gates} />
      </div>

      {#if state.isOk}
        <div class="flex justify-end pt-2">
          <button data-action="continue" class="btn-solid" onclick={onEnter}>Continue →</button>
        </div>
      {/if}
    </div>
  </div>
</div>
