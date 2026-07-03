<script lang="ts">
  import { PageHeader, EmptyState, Eyebrow } from '$lib/components';
  import { KIND_META, type Upgrade, type UpgradeKind } from './buckets.js';

  let { data } = $props();

  const orderedKinds: UpgradeKind[] = ['skill', 'agent', 'rule', 'lint', 'other'];
</script>

{#snippet card(up: Upgrade)}
  <a
    data-upgrade={up.id}
    href={`/project/${up.projectId}/overview`}
    class="block p-4 border border-paper-edge rounded-md bg-paper-soft hover:bg-paper-mute mb-2 text-inherit"
  >
    <div class="flex items-baseline justify-between gap-3 mb-1.5">
      <Eyebrow>{up.projectName}</Eyebrow>
      <span
        class="text-xs uppercase tracking-wide"
        class:text-accent={up.urgency === 'high'}
        class:text-warning={up.urgency === 'medium'}
        class:text-ink-mute={up.urgency === 'low'}
      >{up.urgency}</span>
    </div>
    <p class="text-sm text-ink m-0 leading-snug">{up.title}</p>
    {#if up.why}
      <p class="text-xs text-ink-mute mt-1 m-0 leading-snug">{up.why}</p>
    {/if}
  </a>
{/snippet}

{#snippet bucket(kind: UpgradeKind)}
  {@const meta = KIND_META[kind]}
  {@const items = data.buckets[kind]}
  {#if items.length > 0}
    <section class="mb-8" data-upgrade-bucket={kind}>
      <div class="flex items-baseline gap-3 mb-3">
        <span class="kanji text-2xl">{meta.kanji}</span>
        <h2 class="display text-lg font-normal m-0">{meta.title}</h2>
        <Eyebrow>{items.length}</Eyebrow>
      </div>
      <p class="text-xs text-ink-mute mb-3">{meta.sub}</p>
      <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));">
        {#each items as up (up.id)}
          {@render card(up)}
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

<div class="px-6 pb-10" data-upgrades-total={data.total}>
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
