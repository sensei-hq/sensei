<script lang="ts">
  // Operations · Alerts — the standing alerts across every failure class,
  // each with an ack. Reads the alerts singleton; acking marks, never drops:
  // only the condition clearing removes its alert.
  import { EmptyState } from '@sensei/ui';
  import { Button } from '@rokkit/ui';
  import { alertsState } from '$lib/ops.js';

  const alerts = alertsState;
</script>

<section data-component="ops-alerts" aria-label="Alerts">
  {#if alerts.alerts.length === 0}
    <EmptyState kanji="安" title="All clear" description="No standing alert in any class." />
  {:else}
    <ul>
      {#each alerts.alerts as alert (`${alert.class}:${alert.key}`)}
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
              label="Ack"
              onclick={() => alerts.ack(alert.class, alert.key)}
            />
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
