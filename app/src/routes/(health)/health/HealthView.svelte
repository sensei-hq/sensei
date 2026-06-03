<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import { KanjiHeader, StatusDisc } from '$lib/components';
  import Header from './Header.svelte';
  import Footer from './Footer.svelte';
  import Remedy from './Remedy.svelte';
  import GateRow from './GateRow.svelte';

  interface Props {
    state: HealthState;
    onEnter?: () => void;
    onVerify?: () => void;
  }
  let { state, onEnter, onVerify }: Props = $props();

  const NUMERALS = ['一', '二', '三', '四', '五', '六'] as const;

  const showChecks = $derived(state.status !== 'ok');

  // Overall status for the right-column hero disc.
  // Maps HealthStatus → a ComponentStatus the disc renders.
  const heroDiscStatus = $derived.by(() => {
    if (state.status === 'ok') return 'ready' as const;
    if (state.status === 'needs-action') return 'failed' as const;
    return 'checking' as const;
  });
</script>

<div class="flex-1 min-h-0 overflow-y-auto px-8 py-10">
  <div
    class="w-full mx-auto grid {showChecks ? 'lg:grid-cols-[1fr_1px_1.05fr]' : 'grid-cols-1'} gap-x-7 gap-y-8 min-h-full"
    style="max-width: {showChecks ? '1000px' : '720px'};"
  >
    <!-- Left column · identity, headline, remedy, footer -->
    <div class="flex flex-col min-w-0">
      <Header {state} />

      {#if state.needsAction && state.remedy}
        <Remedy remedy={state.remedy} {onVerify} />
      {/if}

      {#if state.status === 'ok'}
        <div class="mt-5 flex items-center gap-2.5 text-xs text-ink-soft">
          <div class="h-[2px] w-20 bg-success rounded-sm" style="animation: tickle 2.4s ease-in-out infinite;"></div>
          <span class="font-mono tracking-tight">opening…</span>
        </div>
      {/if}

      <div class="mt-auto pt-8">
        <Footer version={state.version} platform={state.platform} />
      </div>
    </div>

    {#if showChecks}
      <!-- Divider -->
      <div class="bg-paper-edge"></div>

      <!-- Right column · hero, ledger -->
      <div class="flex flex-col gap-5 min-w-0">
        <KanjiHeader kanji="支" eyebrow="foundation">
          {#snippet title()}{state.display.heroTitle}{/snippet}
          {#snippet right()}<StatusDisc status={heroDiscStatus} size={32} />{/snippet}
        </KanjiHeader>

        <div class="flex-1 min-h-0 flex flex-col border-t border-paper-edge">
          {#each state.gates as gate, i (gate.id)}
            <GateRow {gate} numeral={NUMERALS[i]} />
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes tickle {
    0%, 100% { transform: scaleX(0.92); opacity: 0.6; }
    50%      { transform: scaleX(1);    opacity: 1; }
  }
</style>
