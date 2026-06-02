<script lang="ts">
  import { sparklinePath } from '$lib/sparkline';
  import { Eyebrow, Kanji, StatusDot } from '$lib/components';
  import RecentSessions from './RecentSessions.svelte';

  let { data } = $props();

  // Mode = listening (no insights/teachings) vs mature (sensei has something to say).
  // Sessions and projects alone don't tip us into mature — the user's guidance:
  // "if we don't have insights it is still in learning phase".
  const mode = $derived<'early' | 'mature'>(
    data.teachings.length === 0 && data.topRecommendations.length === 0
      ? 'early'
      : 'mature',
  );

  let holisticFtr = $derived(
    data.ftrDaily.length > 0
      ? Math.round(data.ftrDaily[data.ftrDaily.length - 1].ftr_rate * 100)
      : 0
  );

  let ftrPrev = $derived(
    data.ftrDaily.length > 1
      ? Math.round(data.ftrDaily[0].ftr_rate * 100)
      : 0
  );

  let ftrDelta = $derived(holisticFtr - ftrPrev);

  let greeting = $derived(() => {
    const h = new Date().getHours();
    if (h < 12) return 'Good morning';
    if (h < 17) return 'Good afternoon';
    return 'Good evening';
  });

  // Early-mode hero body uses concrete numbers from the loader so the page
  // reflects actual progress instead of a static placeholder.
  const earlyBody = $derived.by(() => {
    const n = data.sessionsTotal;
    const p = data.projectFtrs.length;
    if (n === 0 && p === 0) {
      return 'Sensei is ready to watch. Start a session with your assistant — sensei watches in silence, learns the shape of each project, and later begins to teach.';
    }
    if (n === 0) {
      const projects = p === 1 ? '1 project' : `${p} projects`;
      return `Sensei is watching ${projects}. Run a session with your assistant to give it something to learn from.`;
    }
    const sessions = n === 1 ? '1 session' : `${n} sessions`;
    const projects = p === 1 ? '1 project' : `${p} projects`;
    return `Sensei has watched ${sessions} across ${projects} so far. A few early signals are forming, but nothing confident enough to teach yet.`;
  });

  const earlyHint = $derived.by(() => {
    const n = data.sessionsTotal;
    if (n === 0) return null;
    if (n < 5)  return `~${5 - n} more sessions until first lesson`;
    return 'Listening for a confident pattern';
  });
</script>

<div class="max-w-[860px] mx-auto px-12 py-10 pb-16" data-mode={mode}>
  <!-- Greeting + FTR header -->
  <div class="flex items-start justify-between mb-8">
    <div>
      <p class="m-0 mb-2"><Eyebrow>{new Date().toLocaleDateString('en-US', { weekday: 'short', day: 'numeric', month: 'short' })}</Eyebrow></p>
      <h1 class="display text-3xl font-normal m-0 tracking-tight">
        {greeting()}.
      </h1>
    </div>

    {#if mode === 'mature'}
      <div data-ftr-header class="flex items-end gap-5 text-right">
        <div>
          <p class="m-0 mb-1"><Eyebrow>First-Try-Right · 14d</Eyebrow></p>
          <div class="flex items-baseline gap-2 justify-end">
            <span class="display text-3xl font-normal">{holisticFtr}</span>
            <span class="text-xs text-surface-z6">%</span>
            {#if ftrDelta !== 0}
              <span class="mono text-xs" class:ftr-up={ftrDelta > 0} class:ftr-down={ftrDelta < 0}>
                {ftrDelta > 0 ? '↑' : '↓'} {Math.abs(ftrDelta)}%
              </span>
            {/if}
          </div>
        </div>
        {#if data.ftrDaily.length >= 2}
          {@const last = data.ftrDaily[data.ftrDaily.length - 1]}
          {@const vals = data.ftrDaily.map((p: { ftr_rate: number }) => p.ftr_rate)}
          {@const minV = Math.min(...vals)}
          {@const rangeV = Math.max(...vals) - minV || 0.01}
          <svg width="140" height="40" class="block overflow-visible" style="color: oklch(var(--color-primary-z5) / 1);">
            <path d={sparklinePath(data.ftrDaily, 140, 40)} fill="none" stroke="currentColor" stroke-width="1.5" />
            <circle cx="140" cy={40 - (40 - 2) * ((last.ftr_rate - minV) / rangeV) - 1} r="2.5" fill="currentColor" />
          </svg>
        {/if}
      </div>
    {/if}
  </div>

  {#if mode === 'early'}
    <!-- Early / listening hero: 観 + dynamic body with session count -->
    <div data-hero-early class="grid grid-cols-[auto_1fr] gap-7 px-8 py-8 pb-8 bg-surface-z2 border border-surface-z3 rounded-lg mb-8">
      <Kanji char="観" size="4xl" tone="watermark" />
      <div>
        <p class="m-0 mb-3"><Eyebrow>sensei is listening</Eyebrow></p>
        <p class="display text-2xl font-normal m-0 mb-3 tracking-tight">Still listening.</p>
        <p class="text-sm text-surface-z7 leading-normal m-0 max-w-[560px]">
          {earlyBody}
        </p>
        {#if earlyHint}
          <div class="flex items-center gap-2 text-xs text-surface-z6 mt-4">
            <StatusDot status="busy" size="sm" />
            <span>{earlyHint}</span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Early-mode two columns: placeholder insights + empty adopted -->
    <div class="grid grid-cols-[1.4fr_1fr] gap-8 mb-10">
      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-base font-normal m-0">Forming signals</h2>
        </div>
        <div data-early-insights class="flex flex-col">
          <div class="flex gap-3 py-3.5 border-b border-surface-z3 text-left">
            <span class="kanji text-lg w-6 text-surface-z6">耳</span>
            <div class="flex-1">
              <p class="m-0 mb-1"><Eyebrow>listening</Eyebrow></p>
              <p class="text-sm text-surface-z7 leading-snug m-0">
                Watching the shape of your sessions. No confident pattern yet.
              </p>
            </div>
          </div>
          <div class="flex gap-3 py-3.5 border-b border-surface-z3 text-left">
            <span class="kanji text-lg w-6 text-surface-z6">試</span>
            <div class="flex-1">
              <p class="m-0 mb-1"><Eyebrow>calibrating</Eyebrow></p>
              <p class="text-sm text-surface-z7 leading-snug m-0">
                Learning your correction cadence. Too early to suggest rules.
              </p>
            </div>
          </div>
        </div>
      </div>

      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-base font-normal m-0">System has learned</h2>
        </div>
        <div class="py-6 text-center border border-dashed border-surface-z3 rounded-lg">
          <span class="block mb-2"><Kanji char="空" size="2xl" tone="muted" /></span>
          <p class="text-xs text-surface-z6 leading-snug m-0">
            No teachings adopted yet.<br />
            Sensei needs a few more sessions.
          </p>
        </div>
      </div>
    </div>
  {:else}
    <!-- Mature hero: top recommendation -->
    {#if data.topRecommendations.length > 0}
      {@const hero = data.topRecommendations[0]}
      <div data-hero-mature class="grid grid-cols-[auto_1fr] gap-7 px-8 py-8 pb-8 bg-surface-z2 border border-surface-z3 rounded-lg mb-8">
        <Kanji char={hero.urgency === 'high' ? '聴' : hero.urgency === 'medium' ? '繰' : '探'} size="4xl" />
        <div>
          <p class="display text-2xl font-normal m-0 mb-3 tracking-tight">{hero.title}</p>
          <p class="text-sm text-surface-z7 leading-normal m-0 max-w-[520px] mb-4">{hero.why}</p>
          {#if hero.impact}
            <div class="flex items-center gap-2 text-xs">
              <StatusDot status="busy" size="sm" />
              <span class="text-primary-z5">{hero.impact}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Mature two columns: Insights + Adopted teachings -->
    <div class="grid grid-cols-[1.4fr_1fr] gap-8 mb-10">
      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-base font-normal m-0">Also worth noticing</h2>
        </div>
        {#if data.topRecommendations.length > 1}
          {#each data.topRecommendations.slice(1) as rec (rec.id)}
            <div class="flex gap-3 py-3.5 border-b border-surface-z3 text-left">
              <span class="kanji text-lg w-6" class:text-amber={rec.urgency === 'high'} class:text-surface-z6={rec.urgency !== 'high'}>
                {rec.urgency === 'high' ? '繰' : '探'}
              </span>
              <div class="flex-1">
                <p class="m-0 mb-1"><Eyebrow>{rec.urgency}</Eyebrow></p>
                <p class="text-sm text-surface-z7 leading-snug m-0">{rec.title}</p>
              </div>
            </div>
          {/each}
        {:else}
          <div class="py-5 text-center text-sm text-surface-z6 border border-dashed border-surface-z3 rounded-lg">
            No further insights yet.
          </div>
        {/if}
      </div>

      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-base font-normal m-0">System has learned</h2>
        </div>
        {#if data.teachings.length > 0}
          {#each data.teachings as t (t.id)}
            <div class="py-3 px-3.5 mb-2.5 rounded-md bg-surface-z2 border border-surface-z3 border-l-2 border-l-primary-z5">
              <p class="text-sm text-surface-z9 m-0 leading-snug">{t.name}</p>
              <p class="text-xs text-surface-z6 m-0 mt-1">
                {t.family ?? 'pattern'} · {t.instance_count} places
              </p>
            </div>
          {/each}
        {:else}
          <div class="py-6 text-center border border-dashed border-surface-z3 rounded-lg">
            <span class="block mb-2"><Kanji char="空" size="2xl" tone="muted" /></span>
            <p class="text-xs text-surface-z6 leading-snug m-0">
              No teachings adopted yet.<br />
              Sensei needs a few more sessions.
            </p>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Recent sessions — both modes -->
  <RecentSessions sessions={data.recentSessions} />
</div>

<style>
  .ftr-up { color: oklch(var(--color-success-z5) / 1); }
  .ftr-down { color: oklch(var(--color-primary-z5) / 1); }
  .text-amber { color: oklch(0.75 0.15 75); }
</style>
