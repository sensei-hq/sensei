<script lang="ts">
  // Operations · Queue — pending / blocked / backoff listings with waits,
  // exact totals, and per-kind depth. The lists cap daemon-side; `truncated`
  // says when the totals count more than the rows show.
  import { EmptyState } from '@sensei/ui';
  import { jobsState } from '$lib/ops.js';
  import { fmtMs } from '$lib/format.js';

  const jobs = jobsState;

  const empty = $derived(
    jobs.pending.length === 0 && jobs.blocked.length === 0 && jobs.backoff.length === 0,
  );
  const kinds = $derived(Object.entries(jobs.byKind));
</script>

<section data-component="ops-queue" aria-label="Queue">
  <p>pending {jobs.totals.pending} · blocked {jobs.totals.blocked} · running {jobs.totals.running}</p>
  {#if empty}
    <EmptyState kanji="空" title="Queue empty" description="No pending, blocked, or backing-off work." />
  {:else}
    {#if jobs.truncated}
      <p>showing {jobs.pending.length} of {jobs.totals.pending} pending — the daemon capped the list</p>
    {/if}
    {#if jobs.pending.length > 0}
      <ul>
        {#each jobs.pending as row (row.id)}
          <li data-queue-pending>
            <span>{row.kind}</span>
            <span>{row.path}</span>
            <span>{row.wait_ms == null ? 'wait unknown' : `waiting ${fmtMs(row.wait_ms)}`}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if jobs.blocked.length > 0}
      <ul>
        {#each jobs.blocked as row (row.id)}
          <li data-queue-blocked>
            <span>{row.kind}</span>
            <span>{row.path}</span>
            <span>waiting on {row.blocked_on.map((id) => `#${id}`).join(', ')}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if jobs.backoff.length > 0}
      <ul>
        {#each jobs.backoff as row (row.task_id)}
          <li data-queue-backoff>
            <span>{row.kind}</span>
            <span>{row.path}</span>
            <span>attempt {row.attempt}, retrying in {fmtMs(row.retry_in_ms)}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if kinds.length > 0}
      <ul>
        {#each kinds as [kind, depth] (kind)}
          <li data-queue-depth={kind}>{kind}: {depth.pending} pending · {depth.blocked} blocked · {depth.running} running</li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>
