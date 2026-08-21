<script lang="ts">
  import { Button } from '@rokkit/ui';
  import { PageHeader } from '$lib/components';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { IntakeState } from './intake.svelte.js';

  let { data } = $props();

  const api = senseiApi(appState.port);
  // Re-created when the loaded project changes; the state owns recommend/confirm
  // within a project.
  //
  // This was seeded once via `untrack` on the premise that "the id is stable per
  // navigation" — which is false for a `[id]` route. SvelteKit reuses this
  // component when only the param changes, so the intake stayed bound to
  // whichever project mounted first and a playbook run could be recommended
  // against project A while the user was looking at project B. `projectId` is
  // `readonly`, so re-creating is the only way to re-scope.
  //
  // Re-creating also clears `chunk`/`rec`/`phase`, which is what you want: a
  // half-typed description belongs to the project it was typed for.
  const intake = $derived.by(() => new IntakeState(data.guide, data.projectId));

  function recommend(): void { void intake.recommend(api); }
  function confirm(): void { void intake.confirm(api); }
</script>

<div class="flex flex-col gap-3 p-4 max-w-2xl" data-testid="intake">
  <PageHeader kanji="門" eyebrow="This project" title="Intake" description="Start a chunk of work in this project" variant="h1" />

  {#if intake.phase !== 'recorded'}
    <section class="flex flex-col gap-2">
      {#if intake.guide.frame}
        <p class="text-sm text-ink-soft m-0">{intake.guide.frame}</p>
      {/if}
      <textarea
        class="w-full min-h-32 rounded bg-paper-soft border border-paper-edge py-2 px-3 text-sm text-ink"
        placeholder="Describe the work chunk…"
        bind:value={intake.chunk}
        disabled={intake.phase === 'loading'}
        data-testid="intake-input"
      ></textarea>
      <div class="flex justify-end">
        <Button
          variant="primary"
          size="sm"
          onclick={recommend}
          disabled={intake.phase === 'loading' || !intake.chunk.trim()}
          data-testid="intake-recommend"
        >
          {intake.phase === 'loading' ? 'Reading…' : 'Recommend a playbook'}
        </Button>
      </div>
    </section>
  {/if}

  {#if intake.phase === 'error'}
    <p class="text-sm bg-danger-soft text-danger border border-danger rounded py-2 px-3 m-0" data-testid="intake-error">{intake.error}</p>
  {/if}

  {#if intake.rec && (intake.phase === 'recommended' || intake.phase === 'recorded')}
    {@const r = intake.rec}
    <section class="flex flex-col gap-2 rounded bg-paper-soft border border-paper-edge py-2 px-3" data-testid="intake-card">
      <div class="flex items-center justify-between gap-2">
        <h2 class="text-sm font-medium text-ink m-0" data-testid="intake-playbook-title">{intake.playbookTitle}</h2>
        {#if r.auto_select}
          <span class="text-xs bg-success-soft text-success rounded-sm py-1 px-2" data-testid="intake-trust-badge">
            trusted · FTR {r.trust.ftr.toFixed(2)} over {r.trust.n}
          </span>
        {/if}
      </div>
      <p class="text-sm text-ink-soft m-0 leading-snug">{r.rationale}</p>
      {#if r.opening_tone}
        <p class="text-xs italic text-ink-faint m-0">{r.opening_tone}</p>
      {/if}
      <div class="flex flex-wrap gap-2 text-xs text-ink-soft" data-testid="intake-axes">
        <span class="border border-paper-edge rounded-sm py-1 px-2">{r.lifecycle}</span>
        <span class="border border-paper-edge rounded-sm py-1 px-2">{r.intent}</span>
        <span class="border border-paper-edge rounded-sm py-1 px-2">{r.risk}</span>
      </div>

      {#if intake.phase === 'recorded'}
        <div class="flex items-center justify-between gap-2">
          <p class="text-sm text-success m-0" data-testid="intake-recorded">
            {r.auto_select ? 'Auto-selected and recorded.' : 'Recorded.'}
          </p>
          <Button variant="default" style="ghost" size="sm" onclick={() => intake.reset()} data-testid="intake-reset">
            New intake
          </Button>
        </div>
      {:else}
        <div class="flex justify-end">
          <Button variant="primary" size="sm" onclick={confirm} data-testid="intake-confirm">
            Use this playbook
          </Button>
        </div>
      {/if}
    </section>
  {/if}
</div>
