<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import { PageHeader, StatusDisc, GateRow, ProgressCard } from '$lib/components';
  import Header from './Header.svelte';
  import Footer from './Footer.svelte';
  import Remedy from './Remedy.svelte';
  import FoundationNote from './FoundationNote.svelte';

  interface Props {
    state: HealthState;
    /** When true (default), the page auto-navigates on ok and the "opening…"
     *  tickle is shown to telegraph the impending hand-off. When false, the
     *  user is expected to click Continue to proceed (View → Health flow). */
    auto?: boolean;
    onEnter?: () => void;
    onVerify?: () => void;
  }
  let { state, auto = true, onEnter, onVerify }: Props = $props();

  const NUMERALS = ['一', '二', '三', '四', '五', '六'] as const;

  // Maps the overall HealthStatus to a ComponentStatus the right-column hero
  // disc can render. Kept as a derivation here (not in state) because it's a
  // purely presentational concern for the disc's render branch.
  const heroDiscStatus = $derived.by(() => {
    if (state.status === 'ok') return 'ready' as const;
    if (state.status === 'needs-action') return 'failed' as const;
    return 'checking' as const;
  });

  // Coarse install progress for the resolving-state ProgressCard: share of gates
  // already ready. The daemon doesn't emit byte/eta granularity, so those props
  // are omitted (the card degrades to label + bar + count + activity).
  const installPercent = $derived(
    state.total ? Math.round((state.readyCount / state.total) * 100) : 0,
  );
</script>

<div class="flex-1 min-h-0 overflow-y-auto px-8 py-10">
  <div class="w-full max-w-[1000px] mx-auto grid lg:grid-cols-[1fr_1px_1.05fr] gap-x-7 gap-y-8 min-h-full">
    <!-- Left column · identity, headline, remedy, watermark, footer -->
    <div class="relative flex flex-col min-w-0">
      <Header {state} />

      <!-- Left-column body, one per state: blocked → remedy; installing →
           live progress card; checking → "what this is" framing card. The ok
           state shows the watermark + handoff below. -->
      {#if state.needsAction && state.remedy}
        <Remedy remedy={state.remedy} {onVerify} />
      {:else if state.status === 'resolving'}
        <div class="mt-4 max-w-[320px]">
          <ProgressCard
            label="Setting up"
            percent={installPercent}
            left={`${state.readyCount} of ${state.total} ready`}
            activity={state.activeLabel}
            note="Runs once · future launches skip straight through. Nothing leaves your machine."
          />
        </div>
      {:else if !state.isOk}
        <FoundationNote />
      {/if}

      {#if state.status === 'ok' && auto}
        <div class="mt-5 flex items-center gap-2.5 text-xs text-ink-soft">
          <div class="h-[2px] w-20 bg-success rounded-sm" style="animation: tickle 2.4s ease-in-out infinite;"></div>
          <span class="font-mono tracking-tight">opening…</span>
        </div>
      {/if}

      <!-- Watermark — fills the empty space between content and Footer when ok. -->
      <!-- Per updated mockup: container is flex-1 with min-height 80px so it claims -->
      <!-- the spare vertical room; the masked SVG inside sits centered at 62% mask-size -->
      <!-- and 8% opacity, picking up bg-ink so it themes in both light and dark mode. -->
      {#if state.status === 'ok'}
        <div aria-hidden="true" class="flex-1 relative min-h-[80px] mt-4">
          <div class="watermark absolute inset-0 bg-ink opacity-[0.08] pointer-events-none select-none"></div>
        </div>
      {:else}
        <div class="flex-1"></div>
      {/if}

      <div class="pt-2">
        <Footer version={state.version} platform={state.platform} />
      </div>
    </div>

    <!-- Divider -->
    <div class="bg-paper-edge"></div>

    <!-- Right column · hero, ledger -->
    <div class="flex flex-col gap-5 min-w-0">
      <!-- Nested in the right column, which already pads: no chrome, compact glyph. -->
      <PageHeader kanji="支" eyebrow="foundation" size="sm" bordered={false} padded={false}>
        {#snippet title()}{state.display.heroTitle}{/snippet}
        {#snippet right()}<StatusDisc status={heroDiscStatus} size={32} />{/snippet}
      </PageHeader>

      <div class="flex-1 min-h-0 flex flex-col border-t border-paper-edge">
        {#each state.gates as gate, i (gate.id)}
          <GateRow {gate} numeral={NUMERALS[i]} />
        {/each}
      </div>

      {#if state.isOk && !auto}
        <div class="flex justify-end pt-2">
          <button data-action="continue" class="btn-solid btn-sm" onclick={onEnter}>Continue →</button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  @keyframes tickle {
    0%, 100% { transform: scaleX(0.92); opacity: 0.6; }
    50%      { transform: scaleX(1);    opacity: 1; }
  }

  /* Watermark: CSS mask renders the SVG silhouette in the current background
     color (bg-ink). The container is positioned absolute-inset, so size is
     driven by the parent's flex-1 fill; mask-size: 62% keeps the glyph as a
     subtle watermark rather than a wallpaper. */
  .watermark {
    -webkit-mask-image: url('/sensei.svg');
            mask-image: url('/sensei.svg');
    -webkit-mask-size: 62%;
            mask-size: 62%;
    -webkit-mask-repeat: no-repeat;
            mask-repeat: no-repeat;
    -webkit-mask-position: center;
            mask-position: center;
  }
</style>
