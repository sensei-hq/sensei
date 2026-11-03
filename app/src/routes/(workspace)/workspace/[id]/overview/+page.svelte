<script lang="ts">
    import { KIND_ICONS } from '$lib/glyph-vocabulary';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { Kanji, Eyebrow, Screen, ScreenState } from '$lib/components';
  import { Button } from '@rokkit/ui';
  import WorkspacePill from '../../../../(observatory)/workspaces/WorkspacePill.svelte';
  import {
    ftrDisplay,
    workspaceEyebrow,
    repoChipLabel,
    heroContent,
    statBlocks,
    sessionRows,
    hotspotRows,
    patternSnapshot,
    decideRecommendation,
    type StatBlock,
  } from './overview-view.svelte.js';

  let { data } = $props();

  // Lazy: read only inside `{#if data.overview}` below — a failed read shows
  // its error instead of an all-quiet default (#85).
  const workspace = $derived(data.overview!.workspace);
  const rec = $derived(data.overview?.top_recommendation ?? null);

  const ftr = $derived(ftrDisplay(workspace));
  const eyebrow = $derived(workspaceEyebrow(workspace));
  const repoChip = $derived(repoChipLabel(workspace.folders));
  const hero = $derived(heroContent(rec));
  const stats = $derived(statBlocks(data.overview!.stats));
  const rows = $derived(sessionRows(data.overview!.recentSessions));
  const hotspots = $derived(data.hotspots ? hotspotRows(data.hotspots) : []);
  const patterns = $derived(data.patterns ? patternSnapshot(data.patterns) : null);

  const workspaceId = $derived(page.params.id ?? '');

  // Recommendation decision flow (Gap 1 — the accept/reject that gives
  // MeasureVerdicts something to measure). Guards double-clicks with
  // `deciding`, then re-loads so the next scheduler tick sees the update.
  // Retained through the Slot-4 redesign; the mockup's send-to-acp action sits
  // alongside it and appears only when the rec carries a defaultAcp.
  let deciding = $state(false);
  // Why the last decision did not land (#288) — it used to reload as if it had.
  let decideError = $state<string | null>(null);
  async function decide(action: 'accept' | 'reject') {
    if (!rec || !workspaceId || deciding) return;
    deciding = true;
    decideError = null;
    const result = await decideRecommendation(senseiApi(appState.port), workspaceId, rec.id, action, (m) => confirm(m));
    deciding = false;
    if (result.error) decideError = result.error;
    else if (result.done) await invalidateAll();
  }
</script>

{#snippet statBlock(s: StatBlock)}
  <div data-component="stat-card" class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-3">
    <div class="text-xs tracking-wide uppercase text-ink-soft mb-1">{s.label}</div>
    <div class="display text-2xl font-normal leading-none {s.toneClass}">{s.value}</div>
    <div class="text-xs text-ink-soft mt-1">{s.sub}</div>
  </div>
{/snippet}

<Screen>
  {#snippet header()}
    {#if data.overview}
    <!-- Pane header — kanji · eyebrow (+ repo chip) · name · FTR·14d -->
    <header class="flex items-end gap-4 pt-8 px-10 pb-6 max-w-[900px]">
      <!-- The workspace's own mark: an identity, kanji in both modes (#133). -->
      <Kanji char={workspace.kanji} icon={KIND_ICONS.workspace} size="4xl" tone="accent" />
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 mb-1">
          <Eyebrow>{eyebrow}</Eyebrow>
          {#if repoChip}
            <WorkspacePill text={repoChip} />
          {/if}
        </div>
        <h1 class="display text-2xl font-normal m-0 tracking-tight text-ink truncate">
          {workspace.name}
        </h1>
      </div>
      <div class="text-right shrink-0">
        <div class="text-xs tracking-wide uppercase text-ink-soft">FTR · 14d</div>
        <div class="flex items-baseline justify-end gap-1 mt-1">
          {#if ftr.pct === null}
            <span class="display text-2xl font-normal leading-none text-ink-soft">—</span>
          {:else}
            <span class="display text-2xl font-normal leading-none {ftr.toneClass}">{ftr.pct}</span>
            <span class="text-xs text-ink-soft">%</span>
          {/if}
        </div>
      </div>
    </header>
    {/if}
  {/snippet}
  <div class="px-10 pb-12 max-w-[900px]" class:pt-8={!data.overview}>
    {#if !data.overview}
      <ScreenState status="error" error={data.overviewError} onretry={() => invalidateAll()} />
    {:else}

    <!-- Hero — top recommendation, or the all-quiet listening state -->
    <section
      data-testid={hero.quiet ? undefined : 'top-recommendation'}
      class="grid grid-cols-[auto_1fr] gap-5 bg-paper-soft border border-paper-edge rounded-lg px-5 py-5 mb-6"
    >
      <Kanji char={hero.kanji} size="4xl" tone={hero.quiet ? 'watermark' : 'accent'} />
      <div class="min-w-0">
        <div class="text-xs tracking-wide uppercase text-ink-soft mb-1">{hero.eyebrow}</div>
        <p class="display text-xl font-normal leading-snug tracking-tight text-ink m-0 mb-2">
          {hero.headline}
        </p>
        <p class="text-sm text-ink-mute leading-relaxed m-0">{hero.body}</p>

        {#if hero.action || hero.meta || rec}
          <div class="flex items-center gap-3 mt-3 flex-wrap">
            {#if hero.action}
              <Button variant="primary" size="sm">{hero.action} →</Button>
            {/if}
            {#if rec}
              <Button
                variant="primary"
                size="sm"
                data-testid="rec-accept"
                disabled={deciding}
                onclick={() => decide('accept')}
              >{deciding ? 'Working…' : 'Accept'}</Button>
              <Button
                variant="secondary"
                style="outline"
                size="sm"
                data-testid="rec-reject"
                disabled={deciding}
                onclick={() => decide('reject')}
              >Reject</Button>
            {/if}
            {#if hero.meta}
              <span class="font-mono text-xs text-ink-soft">{hero.meta}</span>
            {/if}
          </div>
          {#if decideError}
            <p class="text-xs text-danger mt-2 mb-0" data-testid="rec-decide-error">Could not record that: {decideError}</p>
          {/if}
        {/if}
      </div>
    </section>

    <!-- Stat blocks — sessions · memories · doc drift -->
    <div class="grid grid-cols-3 gap-4 mb-6">
      {#each stats as s (s.key)}
        {@render statBlock(s)}
      {/each}
    </div>

    <!-- Recent in this workspace -->
    <section>
      <div class="flex items-baseline gap-2 mb-3">
        <Kanji char="今" size="sm" tone="accent" />
        <h2 class="display text-sm font-normal uppercase tracking-wide text-ink-mute m-0">
          Recent in this workspace
        </h2>
      </div>

      {#if rows.length > 0}
        <div class="flex flex-col">
          {#each rows as row (row.id)}
            <a
              data-session-row={row.id}
              href={`/workspace/${workspaceId}/sessions#${row.id}`}
              class="grid grid-cols-[1fr_auto] gap-3 items-baseline px-1 py-3 border-b border-paper-edge hover:bg-paper-soft"
            >
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span class="text-sm text-ink truncate">{row.title}</span>
                  {#if row.role}
                    <WorkspacePill text={row.role} />
                  {/if}
                </div>
                <div class="font-mono text-xs text-ink-faint mt-1">{row.meta}</div>
              </div>
              <span class="font-mono text-xs {row.timeToneClass}">{row.time}</span>
            </a>
          {/each}
        </div>
      {:else}
        <p class="text-sm text-ink-mute">No sessions recorded in this workspace yet.</p>
      {/if}
    </section>

    <!-- Hotspots and patterns in use (#85) -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mt-8">
      <section data-testid="overview-hotspots">
        <div class="flex items-baseline gap-2 mb-3">
          <Kanji char="燃" size="sm" tone="accent" />
          <h2 class="display text-sm font-normal uppercase tracking-wide text-ink-mute m-0">
            Hotspots · 7 days
          </h2>
        </div>
        {#if data.hotspotsError}
          <p class="text-sm text-danger m-0" data-overview-error>Could not read hotspots: {data.hotspotsError}</p>
        {:else if hotspots.length === 0}
          <p class="text-sm text-ink-mute m-0">No file edited more than once this week.</p>
        {:else}
          <ul class="list-none m-0 p-0">
            {#each hotspots as h (h.repo + h.path)}
              <li class="grid grid-cols-[1fr_auto] gap-3 items-baseline py-2 border-b border-paper-edge" data-hotspot={h.path}>
                <span class="font-mono text-xs text-ink truncate" title={`${h.repo}/${h.path}`}>{h.path}</span>
                <span class="font-mono text-xs text-ink-soft">{h.edits} edits{#if h.corrections > 0}<span class="text-warning"> · {h.corrections} corrected</span>{/if}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section data-testid="overview-patterns">
        <div class="flex items-baseline gap-2 mb-3">
          <Kanji char="紋" size="sm" tone="accent" />
          <h2 class="display text-sm font-normal uppercase tracking-wide text-ink-mute m-0">
            Patterns in use
          </h2>
        </div>
        {#if data.patternsError}
          <p class="text-sm text-danger m-0" data-overview-error>Could not read patterns: {data.patternsError}</p>
        {:else if patterns && patterns.followed + patterns.anti > 0}
          <a href={`/workspace/${workspaceId}/patterns`} class="block hover:bg-paper-soft rounded-md -mx-1 px-1 py-1">
            <div class="text-sm text-ink" data-pattern-counts>
              {patterns.followed} followed · <span class={patterns.anti > 0 ? 'text-warning' : ''}>{patterns.anti} anti-patterns</span>
            </div>
            {#if patterns.topFollowed.length > 0}
              <div class="font-mono text-xs text-ink-soft mt-2 truncate">{patterns.topFollowed.join(' · ')}</div>
            {/if}
            {#if patterns.topAnti.length > 0}
              <div class="font-mono text-xs text-warning mt-1 truncate">{patterns.topAnti.join(' · ')}</div>
            {/if}
          </a>
        {:else}
          <p class="text-sm text-ink-mute m-0">No patterns detected yet.</p>
        {/if}
      </section>
    </div>
    {/if}
  </div>
</Screen>
