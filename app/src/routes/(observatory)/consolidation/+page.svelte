<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
  import { invalidateAll } from '$app/navigation';
  import { EmptyState, PageHeader, Screen, ScreenState } from '$lib/components';
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
  import { ConsolidationState } from './consolidation-state.svelte.js';

  let { data } = $props();

  const ruleset = $derived<ConsolidatedRuleset | null>(data.ruleset);
  const meta = $derived(ruleset ? statusMeta(ruleset.status) : null);
  const sections = $derived(ruleset ? splitSections(ruleset.content) : []);
  // Source raw-rule count isn't on the consolidated wire shape (it's global
  // scope, resolved daemon-side), so the "rules on disk" delta is omitted.
  const stats = $derived(ruleset ? diffStats(ruleset, null) : []);

  // The actions and their busy / notice state live in ConsolidationState.
  const cs = new ConsolidationState();
  const api = $derived(senseiApi(appState.port));
  const consolidate = () => cs.consolidate(api, invalidateAll);
  const approve = () => (ruleset ? cs.approve(api, ruleset.id, invalidateAll) : undefined);
</script>

<Screen data-consolidation-status={ruleset?.status ?? 'none'}>
  {#snippet header()}
    <PageHeader
      kanji="統"
      icon={railIcon('/consolidation')}
      eyebrow="Governance · Ruleset consolidation"
      title="One coherent ruleset from many raw rules."
      description="Tier-1 gathers every rule governing this scope. Tier-2 asks a model to merge them into one deduped ruleset. Nothing changes until you approve — leaving it unapproved keeps the raw rules separate."
    >
      {#snippet right()}
        {#if ruleset && meta}
          <div class="flex gap-5 pl-5 border-l border-paper-edge">
            <div class="text-center">
              <div class="font-mono text-lg font-light {toneClass(meta.tone)}">v{ruleset.version}</div>
              <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">version</div>
            </div>
            <div class="text-center">
              <div class="font-mono text-lg font-light text-ink">{sections.filter((s) => s.heading).length}</div>
              <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">sections</div>
            </div>
            <div class="text-center">
              <div class="font-mono text-lg font-light {toneClass(meta.tone)}">{meta.label}</div>
              <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">status</div>
            </div>
          </div>
        {/if}
      {/snippet}
    </PageHeader>
  {/snippet}
  {#if data.error}
    <ScreenState status="error" error={data.error} onretry={invalidateAll} />
  {:else if !ruleset}
    <div class="p-6 flex flex-col items-center gap-5">
      <EmptyState
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
        <p class="text-sm text-ink-soft italic m-0" data-notice>{cs.notice}</p>
      {/if}
    </div>
  {:else}
    <div class="pt-5 pb-6 px-7">
      <div class="max-w-[920px]">
        <!-- Eyebrow line — id + model -->
        <div class="flex items-center gap-3 text-xs uppercase tracking-wider text-ink-soft mb-3">
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
                <span class="font-mono text-xs text-ink-mute">{s.before}</span>
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
                <pre class="font-mono text-sm text-ink-soft leading-relaxed whitespace-pre-wrap m-0">{section.body}</pre>
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
            <span class="text-sm text-ink-soft italic" data-notice>{cs.notice}</span>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</Screen>
