<script lang="ts">
  // Operations · Now — what is running right now: live queue rows plus the
  // open transcript/metric/sync waves. Every line reads the singletons; the
  // fault notices read the stores' fault slots, never a hardcoded claim.
  import { Notice, EmptyState } from '@sensei/ui';
  import { waveProgress } from '@sensei/state';
  import { jobsState, syncState } from '$lib/ops.js';
  import { fmtMs } from '$lib/format.js';

  const jobs = jobsState;
  const sync = syncState;

  const unreachable = $derived(!jobs.daemonReachable || !sync.daemonReachable);
  const fault = $derived(jobs.streamFault ?? sync.streamFault);
  const waves = $derived([...jobs.waves.filter((w) => !w.done), ...sync.waves.filter((w) => !w.done)]);
  const idle = $derived(jobs.running.length === 0 && waves.length === 0);
</script>

<section data-component="ops-now" aria-label="Now">
  {#if unreachable}
    <Notice tone="fault" size="inline" expanded title="Daemon unreachable">
      The queue below is the last known state, not the current one.
    </Notice>
  {:else if fault}
    <Notice tone="fault" size="inline" expanded title="Live updates interrupted">
      {fault}
    </Notice>
  {:else if idle}
    <EmptyState kanji="静" title="Nothing running" description="The queue is empty and no pipeline wave is open." />
  {:else}
    {#if jobs.running.length > 0}
      <ul>
        {#each jobs.running as row (row.id)}
          <li data-now-running>
            <span>{row.kind}</span>
            <span>{row.path}</span>
            <span>{row.running_ms == null ? 'just started' : fmtMs(row.running_ms)}</span>
            {#if row.nearing_budget}<span>nearing budget</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if waves.length > 0}
      <ul>
        {#each waves as wave (`${wave.wave}:${wave.key}`)}
          {@const progress = waveProgress(wave)}
          <li data-now-wave>
            <span>{wave.wave}</span>
            {#if progress.share == null}
              <span>gathering totals</span>
            {:else}
              <span>{progress.settled} of {progress.total}</span>
              <span role="progressbar" aria-valuenow={Math.round(progress.share * 100)} aria-valuemin={0} aria-valuemax={100}></span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>
