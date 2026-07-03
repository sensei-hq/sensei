<script lang="ts">
  import { PageHeader, Eyebrow, EmptyState } from '$lib/components';
  import type { TriageRec } from './triage.js';

  let { data } = $props();

  // Typed-descriptor chip mapping — same vocabulary the mockup's
  // RecCardSlim uses. Adds a glyph + short tag so at-a-glance a reader
  // knows what KIND of action this rec is (rule / skill / agent / …)
  // without reading the title.
  const REC_KINDS: Record<string, { glyph: string; tag: string; effect: string }> = {
    write_skill:     { glyph: '技', tag: 'skill',   effect: 'Scaffolds' },
    create_agent:    { glyph: '作', tag: 'agent',   effect: 'Drafts the agent' },
    revise_rule:     { glyph: '則', tag: 'rule',    effect: 'Revises the rule' },
    enrich_memory:   { glyph: '育', tag: 'memory',  effect: 'Reinforces' },
    audit_stale:     { glyph: '禁', tag: 'lint',    effect: 'Audits' },
    promote_pattern: { glyph: '則', tag: 'rule',    effect: 'Promotes the pattern in' },
  };

  function kindFor(actionType: string | null | undefined) {
    if (!actionType) return { glyph: '•', tag: 'suggestion', effect: 'Applies to' };
    return REC_KINDS[actionType] ?? { glyph: '•', tag: actionType, effect: 'Applies to' };
  }

  function relative(iso: string | null): string {
    if (!iso) return '';
    const d = new Date(iso).getTime();
    if (Number.isNaN(d)) return '';
    const diff = Date.now() - d;
    if (diff < 60_000)      return 'just now';
    if (diff < 3_600_000)   return `${Math.round(diff / 60_000)}m ago`;
    if (diff < 86_400_000)  return `${Math.round(diff / 3_600_000)}h ago`;
    return `${Math.round(diff / 86_400_000)}d ago`;
  }
</script>

{#snippet recCard(rec: TriageRec, tone: 'now' | 'soon' | 'settled', focal: boolean)}
  {@const kind = kindFor(rec.actionType)}
  <a
    data-triage-rec={rec.id}
    href={`/project/${rec.projectId}/overview`}
    class="block rounded bg-paper-soft border py-2 px-3 text-inherit mb-2"
    class:border-accent={tone === 'now' && focal}
    class:border-paper-edge={!(tone === 'now' && focal)}
    style={tone === 'now' && focal ? 'border-left: 2px solid var(--accent);' : ''}
  >
    <!-- typed descriptor + (focal) do-first marker -->
    <div class="flex items-center gap-2 mb-1">
      <span class="inline-flex items-center gap-1 text-xs text-ink-soft">
        <span class="kanji text-xs text-accent">{kind.glyph}</span>
        <span class="uppercase tracking-wide">{kind.tag}</span>
      </span>
      <span class="flex-1"></span>
      {#if focal && tone === 'now'}
        <span class="text-xs uppercase tracking-wider text-accent font-medium">do first</span>
      {:else if tone === 'settled' && rec.actedAt}
        <span class="font-mono text-xs text-ink-mute">{rec.status} · {relative(rec.actedAt)}</span>
      {:else}
        <span class="font-mono text-xs text-ink-mute">{rec.projectName}</span>
      {/if}
    </div>

    <p class="text-[13px] text-ink m-0 leading-snug">{rec.title}</p>
    {#if rec.why}
      <p class="text-xs text-ink-soft mt-1 m-0 leading-snug line-clamp-2">{rec.why}</p>
    {/if}

    {#if tone !== 'settled'}
      <div class="text-xs text-ink-faint leading-snug mt-2">
        {kind.effect} <span class="font-mono text-ink-soft">{rec.projectName}</span>
      </div>
    {/if}
  </a>
{/snippet}

{#snippet column(kanji: string, title: string, sub: string, recs: TriageRec[], tone: 'now' | 'soon' | 'settled')}
  <section class="flex flex-col min-w-0 border-r border-paper-edge last:border-r-0" data-triage-column={tone}>
    <!-- Column header — kanji + title + sub + count -->
    <div class="flex items-baseline gap-2 pt-4 pb-3 px-5 border-b border-paper-edge">
      <span
        class="kanji text-[22px] leading-none"
        class:text-accent={tone === 'now'}
        class:text-warning={tone === 'soon'}
        class:text-success={tone === 'settled'}
      >{kanji}</span>
      <div class="flex-1">
        <div class="text-[13px] text-ink font-medium">{title}</div>
        <div class="text-xs text-ink-soft mt-0.5">{sub}</div>
      </div>
      <span
        class="font-mono text-xs"
        class:text-accent={tone === 'now'}
        class:text-warning={tone === 'soon'}
        class:text-success={tone === 'settled'}
      >{recs.length}</span>
    </div>
    <div class="flex flex-col py-3 px-4 gap-0">
      {#if recs.length === 0}
        <p class="text-xs text-ink-faint italic text-center py-6">
          {tone === 'now' ? 'nothing urgent.' : tone === 'soon' ? 'nothing brewing.' : 'no recent decisions.'}
        </p>
      {:else}
        {#each recs as rec, i (rec.id)}
          {@render recCard(rec, tone, tone === 'now' && i === 0)}
        {/each}
      {/if}
    </div>
  </section>
{/snippet}

<PageHeader kanji="學" eyebrow="Observatory · Insights" title="Triage" />
<div class="pb-10">
  {#if data.total === 0}
    <div class="px-6">
      <EmptyState
        kanji="學"
        title="Nothing to triage yet."
        description="Recommendations show up here once the analyzer has enough signal to teach — usually after 20+ sessions across a project."
      />
    </div>
  {:else}
    <div class="grid grid-cols-3" data-triage-grid>
      {@render column('今', 'Now',     'violations · high-impact · this week',  data.buckets.now,     'now')}
      {@render column('近', 'Soon',    'emerging patterns · worth a look',       data.buckets.soon,    'soon')}
      {@render column('定', 'Settled', 'battle-tested · low-noise archive',      data.buckets.settled, 'settled')}
    </div>
  {/if}
</div>
