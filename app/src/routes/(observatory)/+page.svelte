<script lang="ts">
  import { Eyebrow, FtrStrip, Kanji, StatusDot } from '$lib/components';
  import RecentSessions from './RecentSessions.svelte';

  let { data } = $props();

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

  let greeting = $derived.by(() => {
    const h = new Date().getHours();
    if (h < 12) return 'Good morning';
    if (h < 17) return 'Good afternoon';
    return 'Good evening';
  });

  const earlyBody = $derived.by(() => {
    const n = data.sessionsTotal;
    const p = data.projectFtrs.length;
    if (n === 0 && p === 0) {
      return 'sensei is ready to watch. Start a session with your assistant — sensei watches in silence, learns the shape of each project, and later begins to teach.';
    }
    if (n === 0) {
      const projects = p === 1 ? '1 project' : `${p} projects`;
      return `sensei is watching ${projects}. Run a session with your assistant to give it something to learn from.`;
    }
    const sessions = n === 1 ? '1 session' : `${n} sessions`;
    const projects = p === 1 ? '1 project' : `${p} projects`;
    return `sensei has watched ${sessions} across ${projects} so far. A few early signals are forming, but nothing confident enough to teach yet.`;
  });

  const earlyHint = $derived.by(() => {
    const n = data.sessionsTotal;
    if (n === 0) return null;
    if (n < 5)  return `~${5 - n} more sessions until first lesson`;
    return 'Listening for a confident pattern';
  });

  // Map a recommendation's urgency to a hero glyph. Mockup vocabulary:
  // 聴 (listen) for high — sensei is urging action; 繰 (recur) for
  // medium — the same signal keeps repeating; 探 (explore) for low —
  // still investigating.
  function heroKanji(urgency: string): string {
    if (urgency === 'high') return '聴';
    if (urgency === 'medium') return '繰';
    return '探';
  }

  // Relative-time formatter for the hero "noticed" footer + adopted
  // teaching row headers. Falls back to a hyphen for a null iso.
  function relative(iso: string | null | undefined): string {
    if (!iso) return '—';
    const d = new Date(iso).getTime();
    if (Number.isNaN(d)) return '—';
    const diff = Date.now() - d;
    if (diff < 60_000)      return 'just now';
    if (diff < 3_600_000)   return `${Math.round(diff / 60_000)}m ago`;
    if (diff < 86_400_000)  return `${Math.round(diff / 3_600_000)}h ago`;
    return `${Math.round(diff / 86_400_000)}d ago`;
  }
</script>

<div class="max-w-[1060px] mx-auto px-7 py-6 pb-16" data-mode={mode}>
  <!-- Greeting + FTR header -->
  <div class="flex items-start justify-between mb-8">
    <div>
      <p class="m-0 mb-2"><Eyebrow>{new Date().toLocaleDateString('en-US', { weekday: 'short', day: 'numeric', month: 'short' })}</Eyebrow></p>
      <h1 class="display text-3xl font-normal m-0 tracking-tight">
        {greeting}.
      </h1>
    </div>

    {#if mode === 'mature'}
      <div data-ftr-header class="flex items-end gap-5 text-right">
        <div>
          <p class="m-0 mb-1"><Eyebrow>First-Try-Right · 14d</Eyebrow></p>
          <div class="flex items-baseline gap-2 justify-end">
            <span class="display text-3xl font-normal">{holisticFtr}</span>
            <span class="text-xs text-ink-soft">%</span>
            {#if ftrDelta !== 0}
              <span class="font-mono text-xs" class:ftr-up={ftrDelta > 0} class:ftr-down={ftrDelta < 0}>
                {ftrDelta > 0 ? '↑' : '↓'} {Math.abs(ftrDelta)}%
              </span>
            {/if}
          </div>
        </div>
        {#if data.ftrDaily.length >= 2}
          <FtrStrip
            data={data.ftrDaily.map((p: { ftr_rate: number }) => ({ rate: p.ftr_rate }))}
            width={168}
            height={56}
          />
        {/if}
      </div>
    {/if}
  </div>

  {#if mode === 'early'}
    <!-- Early / listening hero: giant 観 + vertical eyebrow + koan -->
    <div data-hero-early class="relative grid grid-cols-[auto_1fr] gap-5 px-6 py-6 bg-paper-mute border border-paper-mute rounded-xl mb-6">
      <div class="relative">
        <span class="kanji block text-[56px] leading-none text-accent opacity-55">観</span>
        <span
          class="absolute -left-1.5 -top-1 text-xs tracking-wider uppercase text-ink-soft"
          style="writing-mode: vertical-rl; transform: rotate(180deg); height: 96px;"
        >sensei is listening</span>
      </div>
      <div class="flex flex-col">
        <p class="display text-2xl font-normal m-0 mb-3 tracking-tight leading-tight">Still listening.</p>
        <p class="text-[15px] text-ink-mute leading-relaxed m-0 mb-4 max-w-[620px]">
          {earlyBody}
        </p>
        {#if earlyHint}
          <div class="flex items-center gap-2 text-xs text-ink-soft mt-auto">
            <StatusDot status="busy" size="sm" />
            <span>{earlyHint}</span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Early-mode two columns: forming signals + empty adopted -->
    <div class="grid grid-cols-[1.4fr_1fr] gap-6 mb-10">
      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-[17px] font-normal m-0 tracking-tight">Early signals</h2>
        </div>
        <div data-early-insights class="flex flex-col">
          <div class="grid grid-cols-[auto_1fr_auto] items-start gap-3 py-3 px-4 border-b border-paper-mute text-left">
            <span class="kanji text-[22px] w-6 text-ink-soft">耳</span>
            <div>
              <p class="m-0 mb-1"><Eyebrow>listening</Eyebrow></p>
              <p class="text-[13px] text-ink-mute leading-snug m-0">
                Watching the shape of your sessions. No confident pattern yet.
              </p>
            </div>
            <span class="font-mono text-xs text-ink-soft px-2 py-1 rounded bg-paper-mute whitespace-nowrap">idle</span>
          </div>
          <div class="grid grid-cols-[auto_1fr_auto] items-start gap-3 py-3 px-4 border-b border-paper-mute text-left">
            <span class="kanji text-[22px] w-6 text-ink-soft">試</span>
            <div>
              <p class="m-0 mb-1"><Eyebrow>calibrating</Eyebrow></p>
              <p class="text-[13px] text-ink-mute leading-snug m-0">
                Learning your correction cadence. Too early to suggest rules.
              </p>
            </div>
            <span class="font-mono text-xs text-ink-soft px-2 py-1 rounded bg-paper-mute whitespace-nowrap">warm-up</span>
          </div>
        </div>
      </div>

      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-[17px] font-normal m-0 tracking-tight">System has learned</h2>
        </div>
        <div class="py-5 px-4 text-center border border-dashed border-paper-edge rounded-lg">
          <span class="block mb-2"><Kanji char="空" size="2xl" tone="muted" /></span>
          <p class="text-[13px] text-ink-soft leading-snug m-0">
            No teachings adopted yet.<br />
            sensei needs a few more sessions to be confident.
          </p>
        </div>
      </div>
    </div>
  {:else}
    <!-- Mature hero: top recommendation composed as a koan card -->
    {#if data.topRecommendations.length > 0}
      {@const hero = data.topRecommendations[0]}
      <div data-hero-mature class="relative grid grid-cols-[auto_1fr] gap-5 px-6 py-6 bg-paper-mute border border-paper-mute rounded-xl mb-6">
        <div class="relative">
          <span class="kanji block text-[56px] leading-none text-accent">{heroKanji(hero.urgency ?? 'medium')}</span>
          <span
            class="absolute -left-1.5 -top-1 text-xs tracking-wider uppercase text-ink-soft"
            style="writing-mode: vertical-rl; transform: rotate(180deg); height: 96px;"
          >sensei speaks</span>
        </div>
        <div class="flex flex-col">
          <p class="display text-2xl font-normal m-0 mb-3 tracking-tight leading-tight">{hero.title}</p>
          <p class="text-[15px] text-ink-mute leading-relaxed m-0 mb-4 max-w-[620px]">{hero.why}</p>
          <div class="flex items-center gap-4 flex-wrap mt-auto">
            {#if hero.impact}
              <div class="flex items-center gap-1 text-[13px] text-accent">
                <span class="w-[5px] h-[5px] rounded-full bg-accent"></span>
                <span>{hero.impact}</span>
              </div>
            {/if}
            <span class="flex-1"></span>
            <span class="font-mono text-xs text-ink-soft">
              {hero.urgency} · noticed {relative(hero.acted_at ?? undefined)}
            </span>
          </div>
        </div>
      </div>
    {/if}

    <!-- Mature two columns: Insights + Adopted teachings -->
    <div class="grid grid-cols-[1.4fr_1fr] gap-6 mb-10">
      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-[17px] font-normal m-0 tracking-tight">Also worth noticing</h2>
          <a href="/insights" class="text-xs text-ink-soft hover:text-ink">all insights →</a>
        </div>
        {#if data.topRecommendations.length > 1}
          <div class="flex flex-col">
            {#each data.topRecommendations.slice(1) as rec (rec.id)}
              {@const tone = rec.urgency === 'high' ? 'warn' : rec.urgency === 'low' ? 'good' : 'ink'}
              <a
                href="/insights"
                class="grid grid-cols-[auto_1fr_auto] items-start gap-3 py-3 px-4 border-b border-paper-mute text-left text-inherit"
                data-tone={tone}
              >
                <span
                  class="kanji text-[22px] w-6"
                  class:text-warning={tone === 'warn'}
                  class:text-success={tone === 'good'}
                  class:text-ink-soft={tone === 'ink'}
                >{heroKanji(rec.urgency ?? 'medium')}</span>
                <div>
                  <p class="m-0 mb-1"><Eyebrow>{rec.urgency}</Eyebrow></p>
                  <p class="text-[13px] text-ink-mute leading-snug m-0">{rec.title}</p>
                </div>
                <span
                  class="font-mono text-xs px-2 py-1 rounded whitespace-nowrap"
                  class:bg-warning-soft={tone === 'warn'}
                  class:text-warning={tone === 'warn'}
                  class:bg-success-soft={tone === 'good'}
                  class:text-success={tone === 'good'}
                  class:bg-paper-mute={tone === 'ink'}
                  class:text-ink-soft={tone === 'ink'}
                >{rec.urgency}</span>
              </a>
            {/each}
          </div>
        {:else}
          <div class="py-4 text-center text-[13px] text-ink-soft italic border border-dashed border-paper-edge rounded-lg">
            Nothing yet. Keep working — sensei watches.
          </div>
        {/if}
      </div>

      <div>
        <div class="flex items-baseline justify-between mb-4">
          <h2 class="display text-[17px] font-normal m-0 tracking-tight">System has learned</h2>
          <a href="/learnings" class="text-xs text-ink-soft hover:text-ink">all teachings →</a>
        </div>
        {#if data.teachings.length > 0}
          <div class="flex flex-col gap-2">
            {#each data.teachings as t (t.id)}
              <div class="py-3 px-3 rounded-md bg-paper-mute border border-paper-mute border-l-2 border-l-accent">
                <div class="flex items-baseline gap-2 mb-1">
                  <span class="font-mono text-xs text-ink-soft">{relative(t.modified_at)}</span>
                  <span class="text-xs text-ink-faint">·</span>
                  <span class="font-mono text-xs text-accent">{t.family ?? 'pattern'}</span>
                </div>
                <p class="text-[13px] text-ink m-0 leading-snug">{t.name}</p>
                <p class="text-xs text-ink-faint m-0 mt-1">
                  {t.instance_count} {t.instance_count === 1 ? 'place' : 'places'}
                </p>
              </div>
            {/each}
          </div>
        {:else}
          <div class="py-5 px-4 text-center border border-dashed border-paper-edge rounded-lg">
            <span class="block mb-2"><Kanji char="空" size="2xl" tone="muted" /></span>
            <p class="text-[13px] text-ink-soft leading-snug m-0">
              No teachings adopted yet.<br />
              sensei needs a few more sessions to be confident.
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
  .ftr-up { color: var(--success); }
  .ftr-down { color: var(--accent); }
</style>
