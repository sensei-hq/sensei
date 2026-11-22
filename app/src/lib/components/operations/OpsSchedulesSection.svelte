<script lang="ts">
  // Operations · Schedules — every worker's cadence and standing, with log
  // tails and PATCH controls on expand. Rows read the schedules singleton;
  // the log and the patch go through the state so the cache and the row
  // update in place.
  import { Notice, EmptyState } from '@sensei/ui';
  import { Button } from '@rokkit/ui';
  import { failureReason, type OpsScheduledTask } from '@sensei/state';
  import { schedulesState } from '$lib/ops.js';

  const schedules = schedulesState;

  let expanded = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let busy = $state(false);
  let intervalDraft = $state('');

  function standing(row: OpsScheduledTask): string {
    if (!row.enabled) return 'paused';
    if (row.running_since) return 'running';
    if (row.last_ok === false) return `failing: ${row.last_error ?? 'no reason recorded'}`;
    if (row.last_skip_reason) return `skipped: ${row.last_skip_reason}`;
    if (row.last_ok === true) return 'healthy';
    return 'never run';
  }

  async function toggle(name: string): Promise<void> {
    if (expanded === name) {
      expanded = null;
      notice = null;
      return;
    }
    expanded = name;
    notice = null;
    const row = schedules.items.find((s) => s.name === name);
    intervalDraft = row ? String(row.interval_secs) : '';
    await schedules.loadLog(name);
  }

  async function setEnabled(name: string, enabled: boolean): Promise<void> {
    notice = null;
    busy = true;
    const res = await schedules.patch(name, { enabled });
    busy = false;
    if (!res.ok) notice = `Change refused: ${failureReason(res.error)}`;
  }

  async function applyInterval(name: string): Promise<void> {
    const secs = Number(intervalDraft);
    if (!Number.isInteger(secs) || secs <= 0) {
      notice = 'Change refused: the cadence is a positive number of seconds';
      return;
    }
    notice = null;
    busy = true;
    const res = await schedules.patch(name, { interval_secs: secs });
    busy = false;
    if (!res.ok) notice = `Change refused: ${failureReason(res.error)}`;
  }
</script>

<section data-component="ops-schedules" aria-label="Schedules">
  {#if schedules.items.length === 0}
    <EmptyState kanji="時" title="No schedules" description="The daemon reports no scheduled workers." />
  {:else}
    <ul>
      {#each schedules.items as row (row.name)}
        {@const log = schedules.logs[row.name]}
        <li>
          <button data-schedule-row={row.name} onclick={() => toggle(row.name)} aria-expanded={expanded === row.name}>
            <span>{row.name}</span>
            <span>every {row.interval_secs}s</span>
            <span>{standing(row)}</span>
          </button>
          {#if expanded === row.name}
            <div data-schedule-detail={row.name}>
              {#if notice}
                <Notice tone="refusal" size="inline" expanded title="Change refused">{notice}</Notice>
              {/if}
              <p>{row.description}</p>
              <Button
                size="sm"
                data-schedule-toggle
                label={row.enabled ? 'Pause' : 'Resume'}
                disabled={busy}
                onclick={() => setEnabled(row.name, !row.enabled)}
              />
              <label>
                Cadence (seconds)
                <input data-schedule-interval type="number" min="1" bind:value={intervalDraft} />
              </label>
              <Button size="sm" data-schedule-apply label="Apply" disabled={busy} onclick={() => applyInterval(row.name)} />
              {#if log}
                <div data-schedule-log>
                  <p>{log.file}</p>
                  <ul>
                    {#each log.lines as line, i (i)}
                      <li>{line}</li>
                    {/each}
                  </ul>
                </div>
              {:else}
                <p>Loading log…</p>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
