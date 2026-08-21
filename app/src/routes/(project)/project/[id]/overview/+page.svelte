<script lang="ts">
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { Kanji, Eyebrow } from '$lib/components';
  import { Button } from '@rokkit/ui';
  import ProjPill from '../../../../(observatory)/projects/ProjPill.svelte';
  import {
    ftrDisplay,
    projectEyebrow,
    repoChipLabel,
    heroContent,
    statBlocks,
    sessionRows,
    type StatBlock,
  } from './overview-view.svelte.js';

  let { data } = $props();

  const project = $derived(data.overview.project);
  const rec = $derived(data.overview.top_recommendation);

  const ftr = $derived(ftrDisplay(project));
  const eyebrow = $derived(projectEyebrow(project));
  const repoChip = $derived(repoChipLabel(project.folders));
  const hero = $derived(heroContent(rec));
  const stats = $derived(statBlocks(data.overview.stats));
  const rows = $derived(sessionRows(data.overview.recentSessions));

  const projectId = $derived(page.params.id ?? '');

  // Recommendation decision flow (Gap 1 — the accept/reject that gives
  // MeasureVerdicts something to measure). Guards double-clicks with
  // `deciding`, then re-loads so the next scheduler tick sees the update.
  // Retained through the Slot-4 redesign; the mockup's send-to-acp action sits
  // alongside it and appears only when the rec carries a defaultAcp.
  let deciding = $state(false);
  async function decide(action: 'accept' | 'reject') {
    if (!rec || !projectId || deciding) return;
    deciding = true;
    try {
      const api = senseiApi(appState.port);
      if (action === 'reject') {
        await api.rejectProjectRecommendation(projectId, rec.id);
      } else {
        // Accept → materialize the rec's artifact:
        //  • rule-class (revise_rule/promote_pattern/enrich_memory) → a governance
        //    rule at project scope / recommended tier (no file — auto).
        //  • write_skill/create_agent → a project FILE (.claude/skills|agents/…).
        //    A file write is consent-sensitive, so confirm (showing the target
        //    path) before writing; declining leaves the rec pending.
        //  • anything else → the plain accept (status flip + FTR measurement).
        const preview = await api.previewRecommendation(projectId, rec.id);
        if (preview.materializable && preview.consent_required) {
          const ok = confirm(
            `Create a ${preview.kind} at ${preview.path}?\n\nsensei will write a new file into this repo (git-tracked, reversible).`,
          );
          if (!ok) return; // declined — rec stays pending
          await api.materializeRecommendation(projectId, rec.id, {});
        } else if (preview.materializable) {
          await api.materializeRecommendation(projectId, rec.id, {
            gov_scope: 'project',
            enforcement: 'recommended',
          });
        } else {
          await api.acceptProjectRecommendation(projectId, rec.id);
        }
      }
      await invalidateAll();
    } finally {
      deciding = false;
    }
  }
</script>

{#snippet statBlock(s: StatBlock)}
  <div data-component="stat-card" class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-3">
    <div class="text-xs tracking-wide uppercase text-ink-mute mb-1">{s.label}</div>
    <div class="display text-2xl font-normal leading-none {s.toneClass}">{s.value}</div>
    <div class="text-xs text-ink-mute mt-1">{s.sub}</div>
  </div>
{/snippet}

<div class="pt-8 px-10 pb-12 max-w-[900px]">
  <!-- Pane header — kanji · eyebrow (+ repo chip) · name · FTR·14d -->
  <header class="flex items-end gap-4 mb-6">
    <Kanji char={project.kanji} size="4xl" tone="accent" />
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-2 mb-1">
        <Eyebrow>{eyebrow}</Eyebrow>
        {#if repoChip}
          <ProjPill text={repoChip} />
        {/if}
      </div>
      <h1 class="display text-2xl font-normal m-0 tracking-tight text-ink truncate">
        {project.name}
      </h1>
    </div>
    <div class="text-right shrink-0">
      <div class="text-xs tracking-wide uppercase text-ink-mute">FTR · 14d</div>
      <div class="flex items-baseline justify-end gap-1 mt-1">
        {#if ftr.pct === null}
          <span class="display text-2xl font-normal leading-none text-ink-mute">—</span>
        {:else}
          <span class="display text-2xl font-normal leading-none {ftr.toneClass}">{ftr.pct}</span>
          <span class="text-xs text-ink-mute">%</span>
        {/if}
      </div>
    </div>
  </header>

  <!-- Hero — top recommendation, or the all-quiet listening state -->
  <section
    data-testid={hero.quiet ? undefined : 'top-recommendation'}
    class="grid grid-cols-[auto_1fr] gap-5 bg-paper-soft border border-paper-edge rounded-lg px-5 py-5 mb-6"
  >
    <Kanji char={hero.kanji} size="4xl" tone={hero.quiet ? 'watermark' : 'accent'} />
    <div class="min-w-0">
      <div class="text-xs tracking-wide uppercase text-ink-mute mb-1">{hero.eyebrow}</div>
      <p class="display text-xl font-normal leading-snug tracking-tight text-ink m-0 mb-2">
        {hero.headline}
      </p>
      <p class="text-sm text-ink-soft leading-relaxed m-0">{hero.body}</p>

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
            <span class="font-mono text-xs text-ink-mute">{hero.meta}</span>
          {/if}
        </div>
      {/if}
    </div>
  </section>

  <!-- Stat blocks — sessions · memories · doc drift -->
  <div class="grid grid-cols-3 gap-4 mb-6">
    {#each stats as s (s.key)}
      {@render statBlock(s)}
    {/each}
  </div>

  <!-- Recent in this project -->
  <section>
    <div class="flex items-baseline gap-2 mb-3">
      <Kanji char="今" size="sm" tone="accent" />
      <h2 class="display text-sm font-normal uppercase tracking-wide text-ink-soft m-0">
        Recent in this project
      </h2>
    </div>

    {#if rows.length > 0}
      <div class="flex flex-col">
        {#each rows as row (row.id)}
          <a
            data-session-row={row.id}
            href={`/project/${projectId}/sessions#${row.id}`}
            class="grid grid-cols-[1fr_auto] gap-3 items-baseline px-1 py-3 border-b border-paper-edge hover:bg-paper-soft"
          >
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-sm text-ink truncate">{row.title}</span>
                {#if row.role}
                  <ProjPill text={row.role} />
                {/if}
              </div>
              <div class="font-mono text-xs text-ink-faint mt-1">{row.meta}</div>
            </div>
            <span class="font-mono text-xs {row.timeToneClass}">{row.time}</span>
          </a>
        {/each}
      </div>
    {:else}
      <p class="text-sm text-ink-soft">No sessions recorded in this project yet.</p>
    {/if}
  </section>
</div>
