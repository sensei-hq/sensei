<script lang="ts">
  // Operations · Alerts — the standing alerts across every failure class,
  // each with an ack. Reads the alerts singleton; acking marks, never drops:
  // only the condition clearing removes its alert. Acks write through to
  // the daemon, so they survive reloads; a refused write says why.
  import { EmptyState, Notice } from '@sensei/ui';
  import { Button } from '@rokkit/ui';
  import { failureReason, OPS_LIST_CAP, type AlertClass } from '@sensei/state';
  import { alertsState } from '$lib/ops.js';

  const alerts = alertsState;

  let notice = $state<string | null>(null);
  let busy = $state(false);

  async function ack(cls: AlertClass, key: string): Promise<void> {
    notice = null;
    busy = true;
    const res = await alerts.ack(cls, key);
    busy = false;
    if (!res.ok) notice = `Ack refused: ${failureReason(res.error)}`;
  }
</script>

<section data-component="ops-alerts" aria-label="Alerts">
  {#if alerts.alerts.length === 0 && !alerts.dispositionsError}
    <EmptyState kanji="安" title="All clear" description="No standing alert in any class." />
  {:else}
    {#if alerts.dispositionsError}
      <Notice tone="fault" size="inline" expanded title="Alert dispositions failing">
        {alerts.dispositionsError}
      </Notice>
    {/if}
    {#if notice}
      <Notice tone="refusal" size="inline" expanded title="Ack refused">{notice}</Notice>
    {/if}
    <ul>
      {#each alerts.alerts.slice(0, OPS_LIST_CAP) as alert (`${alert.class}:${alert.key}`)}
        <li data-alert-row data-alert-level={alert.level}>
          <span>{alert.title}</span>
          <span>{alert.detail}</span>
          <span>{alert.code}</span>
          {#if alert.count > 1}<span>×{alert.count}</span>{/if}
          {#if alert.escalated}<span>Escalated</span>{/if}
          {#if alert.acked}
            <span>Acked</span>
          {:else}
            <Button
              size="sm"
              data-alert-ack={`${alert.class}:${alert.key}`}
              label={busy ? 'Acking…' : 'Ack'}
              disabled={busy}
              onclick={() => ack(alert.class, alert.key)}
            />
          {/if}
        </li>
      {/each}
      {#if alerts.alerts.length > OPS_LIST_CAP}
        <li data-alerts-overflow>showing {OPS_LIST_CAP} of {alerts.alerts.length}</li>
      {/if}
    </ul>
  {/if}
</section>
