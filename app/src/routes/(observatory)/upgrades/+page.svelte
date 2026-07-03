<script lang="ts">
  import { PageHeader, EmptyState, Eyebrow } from '$lib/components';
  import { KIND_META, type Upgrade, type UpgradeKind } from './buckets.js';

  let { data } = $props();

  const orderedKinds: UpgradeKind[] = ['skill', 'agent', 'rule', 'lint', 'other'];

  // Tone per upgrade kind — mirrors the mockup's KIND_META coloring so
  // rule/lint (opinion-shape) reads warning-ish, skill/agent (concrete)
  // reads accent.
  function toneClass(kind: UpgradeKind): string {
    if (kind === 'skill' || kind === 'agent') return 'text-accent';
    if (kind === 'rule')  return 'text-success';
    if (kind === 'lint')  return 'text-warning';
    return 'text-ink-soft';
  }
</script>

{#snippet card(up: Upgrade, kind: UpgradeKind)}
  {@const meta = KIND_META[kind]}
  <a
    data-upgrade={up.id}
    href={`/project/${up.projectId}/overview`}
    class="block bg-paper-soft border border-paper-edge rounded-md py-3 px-4 mb-2 text-inherit hover:bg-paper-mute"
  >
    <!-- typed descriptor + project mono -->
    <div class="flex items-center gap-2 mb-1">
      <span class="inline-flex items-center gap-1 text-xs {toneClass(kind)}">
        <span class="kanji text-xs">{meta.kanji}</span>
        <span class="uppercase tracking-wide">{meta.title}</span>
      </span>
      <span class="flex-1"></span>
      <span class="font-mono text-xs text-ink-mute">{up.projectName}</span>
    </div>

    <!-- title -->
    <p class="text-[13px] text-ink m-0 leading-snug font-medium">{up.title}</p>

    <!-- why with 2-line clamp so the card stays a consistent height -->
    {#if up.why}
      <p class="text-xs text-ink-soft mt-1 m-0 leading-snug line-clamp-2">{up.why}</p>
    {/if}

    <!-- footer: urgency chip + effect caption -->
    <div class="flex items-center gap-2 mt-2 text-xs">
      <span
        class="font-mono px-2 py-0.5 rounded"
        class:bg-warning-soft={up.urgency === 'high'}
        class:text-warning={up.urgency === 'high'}
        class:bg-paper-mute={up.urgency === 'medium'}
        class:text-ink-soft={up.urgency === 'medium'}
        class:bg-paper-mute-alt={up.urgency === 'low'}
        class:text-ink-faint={up.urgency === 'low'}
      >{up.urgency}</span>
      <span class="text-ink-faint">Opens in</span>
      <span class="font-mono text-ink-soft">{up.projectName}</span>
    </div>
  </a>
{/snippet}

{#snippet bucket(kind: UpgradeKind)}
  {@const meta = KIND_META[kind]}
  {@const items = data.buckets[kind]}
  {#if items.length > 0}
    <section class="mb-8" data-upgrade-bucket={kind}>
      <!-- bucket header mirrors the mockup: big kanji + title + count + sub -->
      <div class="flex items-baseline gap-3 mb-1">
        <span class="kanji text-[22px] {toneClass(kind)}">{meta.kanji}</span>
        <h2 class="display text-lg font-normal m-0">{meta.title}</h2>
        <span class="font-mono text-xs {toneClass(kind)}">{items.length}</span>
      </div>
      <p class="text-xs text-ink-soft mb-4">{meta.sub}</p>
      <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));">
        {#each items as up (up.id)}
          {@render card(up, kind)}
        {/each}
      </div>
    </section>
  {/if}
{/snippet}

<PageHeader
  variant="h2"
  eyebrow="Observatory · Upgrades"
  kanji="贈"
  title="Candidates from your own analyzer."
  description="Skills, agents, rules and lints your project's own patterns are asking for. Each links back to the recommendation that produced it."
/>

<div class="max-w-[1060px] mx-auto px-7 pb-10" data-upgrades-total={data.total}>
  {#if data.total === 0}
    <EmptyState
      kanji="贈"
      title="no upgrades waiting"
      description="Recommended skills, agents, rules and lints will appear here once the analyzer has enough evidence — typically after 20+ sessions across a project."
    />
  {:else}
    {#each orderedKinds as kind (kind)}
      {@render bucket(kind)}
    {/each}
  {/if}
</div>
