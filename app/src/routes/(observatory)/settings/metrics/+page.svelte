<script lang="ts">
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { PageHeader, ScreenState, Switch } from '$lib/components';
  import { personaList } from '$lib/personas.svelte.js';
  import { MetricSettings, signedInSlot } from './metric-status-state.svelte.js';

  let { data } = $props();

  // Every derivation, label and write lives in the controller; this stays a pure
  // template. On a LOAD failure `data.summary` is null and the controller is not
  // built — the screen shows error-with-Retry rather than an empty settings page,
  // which would read as "nothing to configure".
  const summary = untrack(() => data.summary);

  // Built with a null persona: the summary comes from `load()` but the persona
  // registry is fetched client-side, so for the first frames this screen genuinely
  // does not know who is signed in. Null renders as "sign in first", which is
  // true, rather than an optimistic guess that fails at the daemon as a 401.
  const state = summary
    ? new MetricSettings(summary, senseiApi(appState.port), null)
    : null;

  // The write credential is the KEYCHAIN SESSION SLOT, and `signedInSlot` takes
  // it from `sessionSlot` — never from `label`, which is a display string.
  $effect(() => {
    void personaList.load().then(() => {
      state?.setPersona(signedInSlot(personaList.personas));
    });
  });

  // The reason kinds map to three treatments, and the mapping is the registry's,
  // not this screen's: `normal` clears itself so it reads as plain text; a
  // `refusal` is somebody's decision; a `fault` needs attention.
  const kindClass = (kind: string) =>
    kind === 'fault' ? 'text-danger' : kind === 'refusal' ? 'text-warning' : 'text-ink-mute';
</script>

<PageHeader
  eyebrow="Settings"
  kanji="測"
  title="Metrics"
  description="Which metrics compute for which repository, how far each has got, and when one is not current, why. Switching a metric off is a cost decision that applies to everyone sharing the repository, so it is recorded by the dōjō that owns it — not here."
/>

{#if data.error || !state}
  <div class="max-w-[1060px] mx-auto px-7 pb-10">
    <ScreenState
      status="error"
      error={data.error ?? 'Failed to load metric status'}
      onretry={invalidateAll}
    />
  </div>
{:else}
  <div class="max-w-[1060px] mx-auto px-7 pb-10 pt-6">
    <!-- Repository rail. The estate arrives aggregated, so this list costs the
         same whether a repository carries 29 metrics or 3,000. -->
    <div class="grid grid-cols-[280px_1fr] gap-7 items-start">
      <nav class="border border-paper-edge rounded-lg overflow-hidden" data-repo-rail>
        {#each state.entries as entry (entry.repository_id)}
          <button
            type="button"
            data-repo={entry.repository_id}
            data-selected={state.selected === entry.repository_id ? 'true' : undefined}
            onclick={() => state.select(entry.repository_id)}
            class="w-full text-left px-3 py-2.5 border-b border-paper-edge last:border-b-0 cursor-pointer
                   hover:bg-paper-soft data-[selected=true]:bg-paper-mute"
          >
            <span class="block text-sm text-ink truncate">{entry.name}</span>
            {#if entry.worst}
              <span class="block text-xs mt-0.5 {kindClass(entry.worst.kind)}">
                {entry.worst.summary}
              </span>
            {/if}
            {#if !entry.configurable}
              <!-- Said here rather than at the toggle, so the reason a control is
                   missing is visible before the click that looks for it. -->
              <span class="block text-xs mt-0.5 text-ink-faint">local only</span>
            {/if}
          </button>
        {/each}
      </nav>

      <section>
        {#if state.loading}
          <ScreenState status="loading" />
        {:else if state.error && state.rows.length === 0}
          <ScreenState
            status="error"
            error={state.error}
            onretry={() => state.selected && state.select(state.selected)}
          />
        {:else if !state.selected}
          <p class="text-sm text-ink-mute m-0">Choose a repository to see its metrics.</p>
        {:else}
          {#if state.error}
            <!-- A write error, with the rows still shown: the reason the switch
                 did not move is more useful beside the switch than in place of it. -->
            <p class="text-xs text-danger mb-3 mt-0" data-write-error>{state.error}</p>
          {/if}
          {#if state.blockedReason}
            <!-- The controller distinguishes the two causes — no remote vs nobody
                 signed in — because one is a fact about the repository and the
                 other is actionable by the reader. -->
            <p class="text-xs text-ink-mute mb-3 mt-0" data-not-configurable>
              {state.blockedReason}
            </p>
          {/if}

          <div class="border border-paper-edge rounded-lg overflow-hidden" data-metric-rows>
            {#each state.rows as row (row.metric)}
              <div
                class="grid grid-cols-[1fr_auto] gap-6 items-start px-4 py-3 border-b
                       border-paper-edge last:border-b-0"
                data-metric={row.metric}
                data-reason={row.reason.code}
              >
                <div>
                  <div class="text-sm text-ink">{row.metric}</div>
                  <div class="text-xs mt-0.5 {kindClass(row.reason.kind)}" data-reason-summary>
                    {row.reason.summary}
                  </div>
                  <div class="text-xs text-ink-faint mt-0.5">
                    {row.cadenceText} · {row.watermarkText} · {row.metric_group}
                  </div>
                  {#if row.reason.remedy}
                    <!-- Only a refusal or a fault carries one — a `normal` code has
                         no remedy by DDL invariant, so nothing is invented here. -->
                    <div class="text-xs text-ink-soft mt-1" data-remedy>{row.reason.remedy}</div>
                  {/if}
                </div>
                <!-- Keyed on the controller's revision so a REFUSED write remounts
                     the switch at the stored value instead of leaving it where it
                     was clicked. -->
                {#key `${row.metric}:${state.revision}`}
                  <Switch
                    value={row.enabled}
                    label={`Toggle ${row.metric} for this repository`}
                    disabled={state.saving || !state.canConfigure}
                    onchange={() => state.toggle(row.metric)}
                  />
                {/key}
              </div>
            {/each}
          </div>

          <p class="text-xs text-ink-faint mt-3 mb-0">
            A change is recorded by the dōjō immediately. The computation state above
            reflects what this machine last synced, so it updates on the next sync.
          </p>
        {/if}
      </section>
    </div>
  </div>
{/if}
