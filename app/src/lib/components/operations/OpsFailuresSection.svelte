<script lang="ts">
  // Operations · Failures — failed tasks with drill-down (attempts +
  // children) and retry, plus abandonments and index errors. The lists read
  // the jobs singleton; drill-down and retry are one-shot client calls with
  // component-local view state. A retry never auto-refreshes: the daemon's
  // log write races the refresh, so the notice names the new id and the
  // Refresh button re-reads on intent.
  import { Notice, EmptyState } from '@sensei/ui';
  import { Button } from '@rokkit/ui';
  import type { OpsQueuedTaskDetail, OpsTaskDetail } from '@sensei/state';
  import { failureReason } from '@sensei/state';
  import { jobsState, opsApi } from '$lib/ops.js';

  const jobs = jobsState;

  let expanded = $state<number | null>(null);
  let detail = $state<OpsTaskDetail | OpsQueuedTaskDetail | null>(null);
  let detailError = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let busy = $state<'detail' | 'retry' | 'refresh' | null>(null);

  const empty = $derived(
    jobs.failed.length === 0 &&
      (jobs.abandoned?.worst.length ?? 0) === 0 &&
      jobs.indexErrors.length === 0,
  );

  async function toggle(id: number): Promise<void> {
    if (expanded === id) {
      expanded = null;
      detail = null;
      detailError = null;
      return;
    }
    expanded = id;
    detail = null;
    detailError = null;
    busy = 'detail';
    const res = await opsApi.tryGetTask(id);
    busy = null;
    if (!res.ok) {
      detailError = failureReason(res.error);
      return;
    }
    detail = res.data;
  }

  async function retry(id: number): Promise<void> {
    notice = null;
    busy = 'retry';
    const res = await opsApi.tryRetryTask(id);
    busy = null;
    if (!res.ok) {
      notice = `Retry refused: ${failureReason(res.error)}`;
      return;
    }
    notice = `Task #${id} re-queued as #${res.data.taskId}`;
  }

  async function refresh(): Promise<void> {
    busy = 'refresh';
    await jobs.refreshFailures();
    busy = null;
  }
</script>

<section data-component="ops-failures" aria-label="Failures">
  {#if !jobs.failuresLoaded}
    <p>Loading failures…</p>
  {:else if empty && !jobs.failuresError}
    <EmptyState kanji="清" title="No failures" description="No failed tasks, abandonments, or index errors in range." />
  {:else}
    {#if notice}
      <Notice tone="normal" size="inline" expanded title="Retry">{notice}</Notice>
    {/if}
    {#if jobs.failuresError}
      <Notice tone="fault" size="inline" expanded title="A failures feed refused">{jobs.failuresError}</Notice>
    {/if}
    {#if jobs.failed.length > 0}
      <ul>
        {#each jobs.failed as row (row.taskId)}
          <li>
            <button data-failure-row={row.taskId} onclick={() => toggle(row.taskId)} aria-expanded={expanded === row.taskId}>
              <span>{row.kind}</span>
              <span>{row.path}</span>
              <span>{row.error ?? 'no reason recorded'}</span>
            </button>
            <Button
              size="sm"
              data-failure-retry={row.taskId}
              label={busy === 'retry' ? 'Retrying…' : 'Retry'}
              disabled={busy !== null}
              onclick={() => retry(row.taskId)}
            />
            {#if expanded === row.taskId}
              <div data-failure-detail={row.taskId}>
                {#if detailError}
                  <span>{detailError}</span>
                {:else if !detail}
                  <span>Loading task…</span>
                {:else if 'attempts' in detail}
                  <p>attempts ({detail.attempts.length})</p>
                  <ul>
                    {#each detail.attempts as attempt (attempt.retryNumber)}
                      <li>#{attempt.retryNumber} {attempt.status}{attempt.error ? ` — ${attempt.error}` : ''}</li>
                    {/each}
                  </ul>
                  <p>
                    children {detail.childSummary.total} (completed {detail.childSummary.completed},
                    failed {detail.childSummary.failed}, running {detail.childSummary.running})
                  </p>
                {:else}
                  <p>still queued — {detail.kind} over {detail.path}</p>
                {/if}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if (jobs.abandoned?.worst.length ?? 0) > 0}
      <ul>
        {#each jobs.abandoned!.worst as row (`${row.task_kind}:${row.path}`)}
          <li data-failure-abandoned>{row.task_kind} {row.path} — abandoned {row.count}×</li>
        {/each}
      </ul>
    {/if}
    {#if jobs.indexErrors.length > 0}
      <ul>
        {#each jobs.indexErrors as row (`${row.folder_id}:${row.file_path}`)}
          <li data-failure-index-error>{row.file_path} — {row.error}</li>
        {/each}
      </ul>
    {/if}
    <Button
      size="sm"
      data-failures-refresh
      label={busy === 'refresh' ? 'Refreshing…' : 'Refresh'}
      disabled={busy !== null}
      onclick={refresh}
    />
  {/if}
</section>
