<script lang="ts">
  // Operations · Sync — dōjō sync state rows (worst-first from the daemon),
  // open pass waves, and transient per-persona plan/push activity. Reads the
  // sync singleton; the remedy text is the daemon's, never a local copy.
  import { EmptyState } from '@sensei/ui';
  import { waveProgress } from '@sensei/state';
  import { syncState } from '$lib/ops.js';

  const sync = syncState;

  const waves = $derived(sync.waves.filter((w) => !w.done));
  const activity = $derived(Object.entries(sync.activity));

  function activityText(stage: 'plan' | 'push', state: 'started' | 'completed' | 'failed'): string {
    if (state === 'failed') return `${stage} failed`;
    if (state === 'completed') return `${stage} done`;
    return stage === 'plan' ? 'planning' : 'pushing';
  }
</script>

<section data-component="ops-sync" aria-label="Sync">
  {#if sync.rows.length === 0 && waves.length === 0 && activity.length === 0}
    <EmptyState kanji="結" title="No sync state" description="The daemon reports no synced entities and no open pass." />
  {:else}
    {#if waves.length > 0}
      <ul>
        {#each waves as wave (`${wave.wave}:${wave.key}`)}
          {@const progress = waveProgress(wave)}
          <li data-sync-wave>
            <span>pass {wave.key}</span>
            {#if progress.share == null}
              <span>gathering totals</span>
            {:else}
              <span>{progress.settled} of {progress.total}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if activity.length > 0}
      <ul>
        {#each activity as [persona, entry] (persona)}
          <li data-sync-activity={persona}>
            <span>{persona}</span>
            <span>{activityText(entry.stage, entry.state)}</span>
            {#if entry.error}<span>{entry.error}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if sync.rows.length > 0}
      <ul>
        {#each sync.rows as row (`${row.entity}:${row.entity_key}`)}
          <li data-sync-row>
            <span>{row.entity_key}</span>
            <span>{row.state}</span>
            {#if row.last_error}<span>{row.last_error}</span>{/if}
            {#if row.remedy}<span>{row.remedy}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if sync.waveHistory.length > 0}
      <p>Recent passes</p>
      <ul>
        {#each sync.waveHistory as run (run.id)}
          <li data-sync-history>
            <span>pass {run.key}</span>
            <span>{run.settled} of {run.total}</span>
            <span>{run.outcome}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>
