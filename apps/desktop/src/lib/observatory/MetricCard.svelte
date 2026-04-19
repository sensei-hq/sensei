<script lang="ts">
  import type { MetricValue } from './types.js';

  let { label, metric, format: fmt = (v: number) => String(v), trend, trendLabel }: {
    label: string;
    metric: MetricValue;
    format?: (v: number) => string;
    trend?: 'up' | 'down' | null;
    trendLabel?: string;
  } = $props();

  const trendColor = $derived(
    trend === 'up' ? 'text-success-z6' : trend === 'down' ? 'text-error-z6' : 'text-surface-z4'
  );
</script>

<div class="rounded-lg bg-surface-z2 p-3">
  <p class="text-[10px] text-surface-z5 uppercase tracking-wide font-medium">{label}</p>

  {#if metric.quality === 'unavailable'}
    <p class="mt-1 text-xl font-semibold text-surface-z3">&mdash;</p>
    {#if metric.trackingUrl}
      <a href={metric.trackingUrl} target="_blank" class="text-[9px] text-primary-z5 hover:text-primary-z6">Track issue</a>
    {/if}
  {:else}
    <div class="mt-1 flex items-baseline gap-1.5">
      <p class="text-xl font-semibold {metric.quality === 'estimated' ? 'text-surface-z6' : 'text-primary-z6'}">
        {fmt(metric.value as number)}
      </p>
      {#if metric.quality === 'estimated'}
        <span class="rounded bg-warning-z2 px-1 py-0.5 text-[8px] font-medium text-warning-z7">est.</span>
      {/if}
    </div>
    {#if trendLabel}
      <p class="mt-0.5 text-[10px] {trendColor}">{trendLabel}</p>
    {/if}
    {#if metric.hint}
      <p class="mt-0.5 text-[9px] text-surface-z4">{metric.hint}</p>
    {/if}
  {/if}
</div>
