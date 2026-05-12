<script lang="ts">
  let { data } = $props();

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

  let hasData = $derived(data.ftrDaily.length > 0 || data.projectFtrs.length > 0);

  let greeting = $derived(() => {
    const h = new Date().getHours();
    if (h < 12) return 'Good morning';
    if (h < 17) return 'Good afternoon';
    return 'Good evening';
  });

  // SVG sparkline from ftr_daily data
  function sparklinePath(points: Array<{ ftr_rate: number }>, w: number, h: number): string {
    if (points.length < 2) return '';
    const vals = points.map(p => p.ftr_rate);
    const min = Math.min(...vals);
    const max = Math.max(...vals);
    const range = max - min || 0.01;
    const step = w / (vals.length - 1);
    return vals.map((v, i) => {
      const x = i * step;
      const y = h - (h - 2) * ((v - min) / range) - 1;
      return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)} ${y.toFixed(1)}`;
    }).join(' ');
  }
</script>

<div class="max-w-[860px] mx-auto px-12 py-10 pb-16">
  <!-- Greeting + FTR header -->
  <div class="flex items-start justify-between mb-8">
    <div>
      <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-2">
        {new Date().toLocaleDateString('en-US', { weekday: 'short', day: 'numeric', month: 'short' })}
      </p>
      <h1 class="display text-3xl font-normal m-0 tracking-tight">
        {greeting()}.
      </h1>
    </div>

    {#if hasData}
      <div class="flex items-end gap-5 text-right">
        <div>
          <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-1">
            First-Try-Right · 14d
          </p>
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
          {@const vals = data.ftrDaily.map(p => p.ftr_rate)}
          {@const minV = Math.min(...vals)}
          {@const rangeV = Math.max(...vals) - minV || 0.01}
          <svg width="140" height="40" class="block overflow-visible" style="color: oklch(var(--color-primary-z5) / 1);">
            <path d={sparklinePath(data.ftrDaily, 140, 40)} fill="none" stroke="currentColor" stroke-width="1.5" />
            <circle
              cx="140"
              cy={40 - (40 - 2) * ((last.ftr_rate - minV) / rangeV) - 1}
              r="2.5"
              fill="currentColor"
            />
          </svg>
        {/if}
      </div>
    {/if}
  </div>

  {#if !hasData}
    <!-- Early / listening state -->
    <div class="grid grid-cols-[auto_1fr] gap-7 px-8.5 py-8 pb-7.5 bg-surface-z2 border border-surface-z3 rounded-lg mb-8">
      <span class="kanji text-7xl text-primary-z5 opacity-55 leading-none">観</span>
      <div>
        <p class="display text-2xl font-normal m-0 mb-3 tracking-tight">Still listening.</p>
        <p class="text-sm text-surface-z7 leading-relaxed m-0 max-w-[520px]">
          Sensei is watching your sessions. A few early signals are forming,
          but nothing confident enough to teach yet.
        </p>
      </div>
    </div>
    <p class="text-xs text-surface-z6 leading-reading max-w-[480px]">
      Start a session with your assistant. Sensei watches in silence,
      learns the shape of each project, and later begins to teach.
    </p>
  {:else}
    <!-- Hero: top recommendation -->
    {#if data.topRecommendations.length > 0}
      {@const hero = data.topRecommendations[0]}
      <div class="grid grid-cols-[auto_1fr] gap-7 px-8.5 py-8 pb-7.5 bg-surface-z2 border border-surface-z3 rounded-lg mb-8">
        <span class="kanji text-7xl text-primary-z5 leading-none">
          {hero.urgency === 'high' ? '聴' : hero.urgency === 'medium' ? '繰' : '探'}
        </span>
        <div>
          <p class="display text-2xl font-normal m-0 mb-3 tracking-tight">{hero.title}</p>
          <p class="text-sm text-surface-z7 leading-relaxed m-0 max-w-[520px] mb-4">{hero.why}</p>
          {#if hero.impact}
            <div class="flex items-center gap-2 text-xs">
              <span class="w-1.5 h-1.5 rounded-full bg-primary-z5"></span>
              <span class="text-primary-z5">{hero.impact}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Two columns: Insights + Adopted teachings -->
    <div class="grid grid-cols-[1.4fr_1fr] gap-8 mb-10">
      <!-- Insights (remaining recommendations) -->
      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-base font-normal m-0">Also worth noticing</h2>
        </div>
        {#if data.topRecommendations.length > 1}
          {#each data.topRecommendations.slice(1) as rec (rec.id)}
            <div class="flex gap-3 py-3.5 border-b border-surface-z3 text-left">
              <span class="kanji text-lg w-6.5" class:text-amber={rec.urgency === 'high'} class:text-surface-z6={rec.urgency !== 'high'}>
                {rec.urgency === 'high' ? '繰' : '探'}
              </span>
              <div class="flex-1">
                <p class="text-xs tracking-loose uppercase text-surface-z6 m-0 mb-1">{rec.urgency}</p>
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

      <!-- Adopted teachings -->
      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-base font-normal m-0">System has learned</h2>
        </div>
        {#if data.teachings.length > 0}
          {#each data.teachings as t (t.id)}
            <div class="py-3 px-3.5 mb-2.5 rounded-md bg-surface-z2 border border-surface-z3 border-l-2 border-l-primary-z5">
              <p class="text-sm text-surface-z9 m-0 leading-snug">{t.name}</p>
              <p class="text-2xs text-surface-z6 m-0 mt-1">
                {t.family ?? 'pattern'} · {t.instance_count} places
              </p>
            </div>
          {/each}
        {:else}
          <div class="py-6 text-center border border-dashed border-surface-z3 rounded-lg">
            <span class="kanji text-2xl text-surface-z5 block mb-2">空</span>
            <p class="text-xs text-surface-z6 leading-snug m-0">
              No teachings adopted yet.<br />
              Sensei needs a few more sessions.
            </p>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .ftr-up { color: oklch(var(--color-success-z5) / 1); }
  .ftr-down { color: oklch(var(--color-primary-z5) / 1); }
  .text-amber { color: oklch(0.75 0.15 75); }
</style>
