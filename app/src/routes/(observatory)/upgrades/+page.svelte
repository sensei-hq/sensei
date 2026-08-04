<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { PageHeader, EmptyState } from '$lib/components';
  import { Button } from '@rokkit/ui';
  import type { DojoUpgrade } from '$lib/types.js';
  import { KIND_META, type Upgrade, type UpgradeKind } from './buckets.js';
  import {
    UpgradeActions,
    attributionSummary,
    orderUpgrades,
    receivedLabel,
    scopeSummary,
    stateChip,
    typePill,
    type ChipClasses,
  } from './dojo-upgrades-state.svelte.js';

  let { data } = $props();

  const orderedKinds: UpgradeKind[] = ['skill', 'agent', 'rule', 'lint', 'other'];

  // The Dōjō lane's Apply / Mute / Pin controller — the api client + reload
  // callback are injected so the component stays a pure template; success
  // re-loads the list, a failure surfaces `actions.error`.
  const actions = new UpgradeActions(senseiApi(appState.port), invalidateAll);

  // Pinned-first ordering mirrors the daemon; derived so it re-sorts on reload.
  const orderedUpgrades = $derived(orderUpgrades(data.dojoUpgrades));

  // Tone per LOCAL upgrade kind — mirrors the mockup's KIND_META coloring so
  // rule/lint (opinion-shape) reads warning-ish, skill/agent (concrete)
  // reads accent.
  function toneClass(kind: UpgradeKind): string {
    if (kind === 'skill' || kind === 'agent') return 'text-accent';
    if (kind === 'rule')  return 'text-success';
    if (kind === 'lint')  return 'text-warning';
    return 'text-ink-soft';
  }
</script>

{#snippet chip(cls: ChipClasses)}
  <span class="font-mono text-xs px-2 py-0.5 rounded-full {cls.bg} {cls.text}">{cls.label}</span>
{/snippet}

<!-- Dōjō downstream artifact card — type pill · state chip · attribution · scope · actions -->
{#snippet upgradeCard(up: DojoUpgrade)}
  {@const pill = typePill(up.type)}
  {@const received = receivedLabel(up.received_at)}
  <article
    data-upgrade={up.id}
    data-state={up.state}
    class="border rounded-lg bg-paper-soft p-4 mb-3"
    class:border-accent-soft={up.state === 'pinned'}
    class:border-paper-edge={up.state !== 'pinned'}
  >
    <!-- header: type pill · state chip · dereferenced badge · received-at -->
    <div class="flex items-center gap-2 mb-2">
      <span class="inline-flex items-center gap-1 font-mono text-xs px-2 py-0.5 rounded-full {pill.bg} {pill.text}">
        <span class="kanji">{pill.kanji}</span>
        <span class="uppercase tracking-wide">{pill.label}</span>
      </span>
      {@render chip(stateChip(up.state))}
      {#if up.attribution?.dereferenced}
        {@render chip({ bg: 'bg-paper-mute', text: 'text-ink-mute', label: 'dereferenced' })}
      {/if}
      <span class="flex-1"></span>
      {#if received}
        <span class="font-mono text-xs text-ink-faint">{received}</span>
      {/if}
    </div>

    <!-- title + body (insight-copy) -->
    <p class="text-sm text-ink m-0 leading-snug font-medium">{up.title}</p>
    {#if up.body}
      <p class="text-xs text-ink-soft mt-1 m-0 leading-snug line-clamp-3">{up.body}</p>
    {/if}

    <!-- provenance: scope + attribution -->
    <div class="flex flex-wrap items-center gap-x-4 gap-y-1 mt-2 text-xs text-ink-mute">
      <span data-scope>{scopeSummary(up.scope)}</span>
      <span data-attribution>{attributionSummary(up.attribution)}</span>
    </div>

    <!-- deferred / scope-mismatch note, when the daemon left one -->
    {#if up.note}
      <p class="text-xs text-ink-faint mt-2 m-0 leading-snug">{up.note}</p>
    {/if}

    <!-- one-decision-default: Apply · then Pin / Mute -->
    <div class="flex items-center gap-2 mt-3 pt-3 border-t border-paper-edge">
      <Button
        variant="primary"
        size="sm"
        data-action="apply"
        disabled={actions.isBusy(up.id)}
        onclick={() => actions.apply(up.id)}
      >Apply</Button>
      <Button
        variant="secondary"
        style="outline"
        size="sm"
        data-action="pin"
        disabled={actions.isBusy(up.id)}
        onclick={() => actions.pin(up.id)}
      >Pin</Button>
      <Button
        variant="secondary"
        style="outline"
        size="sm"
        data-action="mute"
        disabled={actions.isBusy(up.id)}
        onclick={() => actions.mute(up.id)}
      >Mute</Button>
    </div>
  </article>
{/snippet}

<!-- Local analyzer recommendation card (unchanged) -->
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
  title="Upgrades waiting for your call."
  description="Practice arriving from your Dōjō, and candidate skills, agents, rules and lints your own analyzer is asking for. Review each, then apply it or set it aside."
/>

<div
  class="max-w-[1060px] mx-auto px-7 pb-10"
  data-upgrades-total={data.total}
  data-dojo-total={data.dojoUpgrades.length}
>
  <!-- Lane 1 · the Dōjō downstream inbox -->
  <section class="mb-10" data-lane="dojo">
    <div class="flex items-baseline gap-3 mb-1">
      <h2 class="display text-lg font-normal m-0">From your Dōjō</h2>
      {#if data.dojoUnread > 0}
        <span class="font-mono text-xs text-accent" data-dojo-unread>{data.dojoUnread} new</span>
      {/if}
    </div>
    <p class="text-xs text-ink-soft mb-4">
      Approved practice pulled back from the teams you've paired with. Applying lands it into your
      local rules and memory.
    </p>

    {#if actions.error}
      <p class="text-xs text-danger mb-3" data-dojo-error>{actions.error}</p>
    {/if}

    {#if orderedUpgrades.length === 0}
      <EmptyState
        kanji="贈"
        title="no dōjō upgrades yet"
        description="Connect a Dōjō to receive shared learnings. Approved principles, patterns, skills and guards land here for you to apply."
      />
    {:else}
      {#each orderedUpgrades as up (up.id)}
        {@render upgradeCard(up)}
      {/each}
    {/if}
  </section>

  <!-- Lane 2 · candidates from the local analyzer (behaviour unchanged) -->
  <section data-lane="local">
    <div class="flex items-baseline gap-3 mb-1">
      <h2 class="display text-lg font-normal m-0">From your analyzer</h2>
      {#if data.total > 0}
        <span class="font-mono text-xs text-ink-soft">{data.total}</span>
      {/if}
    </div>
    <p class="text-xs text-ink-soft mb-4">
      Candidate skills, agents, rules and lints your project's own patterns are asking for. Each
      links back to the recommendation that produced it.
    </p>

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
  </section>
</div>
