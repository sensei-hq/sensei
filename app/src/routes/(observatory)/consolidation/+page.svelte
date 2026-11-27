<script lang="ts">
    import { EmptyState, StateView } from '@sensei/ui';
    import Kanji from '$lib/components/Kanji.svelte';
  import { invalidateAll } from '$app/navigation';
  import { ShellScreen } from '@sensei/ui';
  import type { ShellStat } from '@rokkit/app';
  import { railIcon } from '../observatory-nav';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import {
    statusMeta,
    toneClass,
    splitSections,
    diffStats,
    type ConsolidatedRuleset,
  } from './consolidation-view.js';
  import { consolidationState } from './consolidation-state.svelte.js';

  let { data } = $props();

  const ruleset = $derived<ConsolidatedRuleset | null>(data.ruleset);
  const meta = $derived(ruleset ? statusMeta(ruleset.status) : null);
  const sections = $derived(ruleset ? splitSections(ruleset.content) : []);
  // The header's version/sections/status trio, as the shell's stats band —
  // absent until a ruleset reads (the band hides with it).
  const cards = $derived<ShellStat[] | undefined>(
    ruleset && meta
      ? [
          { id: 'version', title: 'Version', value: `v${ruleset.version}`, tone: meta.tone },
          { id: 'sections', title: 'Sections', value: sections.filter((s) => s.heading).length, tone: 'ink' as const },
          { id: 'status', title: 'Status', value: meta.label, tone: meta.tone },
        ]
      : undefined,
  );
  // Source raw-rule count isn't on the consolidated wire shape (it's global
  // scope, resolved daemon-side), so the "rules on disk" delta is omitted.
  const stats = $derived(ruleset ? diffStats(ruleset, null) : []);

  // The actions and their busy / notice state live in the module
  // singleton, so the guard survives navigation mid-run.
  const cs = consolidationState;
  const api = $derived(senseiApi(appState.port));
  const consolidate = () => cs.consolidate(api, invalidateAll);
  const approve = () => (ruleset ? cs.approve(api, ruleset.id, invalidateAll) : undefined);
</script>

<!-- On the ScreenShell (D87 §9): the header's mark/title/eyebrow/description are
     the shell's, the version/sections/status trio is its stats band, the body
     moves in unchanged. -->
<ShellScreen
  viewId="observatory.consolidation"
  mark={{ kanji: '統', icon: railIcon('/consolidation') }}
  title="One coherent ruleset from many raw rules."
  context="Governance · Ruleset consolidation"
  description="Tier-1 gathers every rule governing this scope. Tier-2 asks a model to merge them into one deduped ruleset. Nothing changes until you approve — leaving it unapproved keeps the raw rules separate."
  search={false}
  stats={cards}
  data-consolidation-status={ruleset?.status ?? 'none'}
>
  {#if data.error}
    <StateView status="error" error={data.error} failure={data.failure} onretry={invalidateAll} />
  {:else if !ruleset}
    <div class="p-6 flex flex-col items-center gap-5">
      <EmptyState size="screen"
        kanji="結"
        title="nothing consolidated yet"
        description="When rules start to overlap, sensei can merge them into one coherent ruleset here. Run a consolidation to propose a merge, then approve it to keep."
      />
      <button
        type="button"
        class="zs-btn zs-btn-primary"
        data-action="consolidate"
        disabled={cs.busy !== null}
        onclick={consolidate}
      >
        {cs.busy === 'consolidate' ? 'consolidating…' : 'consolidate now'}
      </button>
      {#if cs.notice}
        <p class="text-sm text-ink-mute italic m-0" data-notice>{cs.notice}</p>
      {/if}
    </div>
  {:else}
    <div class="pt-5 pb-6 px-7">
      <div class="max-w-[920px]">
        <!-- Eyebrow line — id + model -->
        <div class="flex items-center gap-3 text-xs uppercase tracking-wider text-ink-mute mb-3">
          <span>Consolidated ruleset</span>
          <span class="text-ink-faint">·</span>
          <span class="font-mono normal-case tracking-normal">{ruleset.id}</span>
          {#if ruleset.model}
            <span class="text-ink-faint">·</span>
            <span class="font-mono normal-case tracking-normal">{ruleset.model}</span>
          {/if}
        </div>

        <!-- Before -> after diff strip -->
        <div class="grid grid-cols-4 gap-[1px] bg-paper-edge rounded overflow-hidden mb-5">
          {#each stats as s (s.label)}
            <div class="bg-paper-mute py-3 px-4">
              <div class="text-xs uppercase tracking-wider text-ink-faint mb-1">{s.label}</div>
              <div class="flex items-baseline gap-2">
                <span class="font-mono text-xs text-ink-soft">{s.before}</span>
                <span class="text-xs text-ink-faint">→</span>
                <span class="display text-lg font-normal {toneClass(s.tone)}">{s.after}</span>
                {#if s.delta}
                  <span class="font-mono text-xs ml-auto {toneClass(s.tone)}">{s.delta}</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        <!-- Proposed merged ruleset — sectioned markdown, the "after" -->
        <div class="flex items-center gap-1 mb-3">
          <span class="text-xs uppercase tracking-wider text-accent font-medium">
            {meta?.approvable ? 'Proposed merged ruleset' : 'Merged ruleset'}
          </span>
          <Kanji char="新" size="inherit" tone="inherit" class="text-sm text-accent" />
        </div>

        <div class="bg-paper-soft border border-accent rounded-lg py-3 px-4 mb-5">
          {#each sections as section, i (i)}
            <div class:mt-4={i > 0}>
              {#if section.heading}
                <h3 class="display text-base font-medium text-ink m-0 mb-1">{section.heading}</h3>
              {/if}
              {#if section.body}
                <pre class="font-mono text-sm text-ink-mute leading-relaxed whitespace-pre-wrap m-0">{section.body}</pre>
              {/if}
            </div>
          {/each}
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-2 pt-1">
          {#if meta?.approvable}
            <button
              type="button"
              class="zs-btn zs-btn-primary"
              data-action="approve"
              disabled={cs.busy !== null}
              onclick={approve}
            >
              {cs.busy === 'approve' ? 'approving…' : 'approve · merge & keep'}
            </button>
          {/if}
          <button
            type="button"
            class="zs-btn zs-btn-secondary"
            data-action="consolidate"
            disabled={cs.busy !== null}
            onclick={consolidate}
          >
            {cs.busy === 'consolidate' ? 'consolidating…' : 're-consolidate'}
          </button>
          <span class="flex-1"></span>
          {#if cs.notice}
            <span class="text-sm text-ink-mute italic" data-notice>{cs.notice}</span>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</ShellScreen>
