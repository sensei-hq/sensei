<script lang="ts">
  // Operations · Pipelines — transcript/metric waves with progress, and the
  // transcript backfill trigger. Waves read the jobs singleton; the backfill
  // is a one-shot client call. Sync passes live in their own section.
  import { Notice, EmptyState } from '@sensei/ui';
  import { Button } from '@rokkit/ui';
  import { failureReason, waveProgress } from '@sensei/state';
  import { jobsState, opsApi } from '$lib/ops.js';

  const jobs = jobsState;

  let notice = $state<string | null>(null);
  let busy = $state(false);

  async function backfill(): Promise<void> {
    notice = null;
    busy = true;
    const res = await opsApi.tryStartBackfill();
    busy = false;
    if (!res.ok) {
      notice = `Backfill refused: ${failureReason(res.error)}`;
      return;
    }
    if (res.data.queued && typeof res.data.taskId === 'number') {
      notice = `Backfill started as #${res.data.taskId}`;
    } else {
      notice = 'A backfill is already running — its wave is listed above';
    }
  }
</script>

<section data-component="ops-pipelines" aria-label="Pipelines">
  {#if notice}
    <Notice tone="normal" size="inline" expanded title="Backfill">{notice}</Notice>
  {/if}
  {#if jobs.waves.length === 0}
    <EmptyState kanji="流" title="No pipeline waves" description="No transcript or metric wave is open." />
  {:else}
    <ul>
      {#each jobs.waves as wave (`${wave.wave}:${wave.key}`)}
        {@const progress = waveProgress(wave)}
        <li data-pipeline-wave>
          <span>{wave.wave}</span>
          {#if progress.share == null}
            <span>gathering totals</span>
          {:else}
            <span>{progress.settled} of {progress.total}</span>
            {#if wave.done}<span>done</span>{/if}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  {#if jobs.waveHistory.length > 0}
    <p>Recent waves</p>
    <ul>
      {#each jobs.waveHistory as run (run.id)}
        <li data-pipeline-history>
          <span>{run.wave}</span>
          <span>{run.settled} of {run.total}</span>
          <span>{run.outcome}</span>
        </li>
      {/each}
    </ul>
  {/if}
  <Button
    size="sm"
    data-pipeline-backfill
    label={busy ? 'Starting…' : 'Run transcript backfill'}
    disabled={busy}
    onclick={backfill}
  />
</section>
