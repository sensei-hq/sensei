<script lang="ts">
  import { PageHeader } from '$lib/components';
  let { data } = $props();
</script>

<PageHeader title="Sessions" description="{data.ftrMetrics?.sessions7d ?? 0} in last 7 days" />
<div class="px-6 py-6">
  <ul class="list-none m-0 p-0">
    {#each data.sessions as s (s.id)}
      <li class="session-row flex gap-3 py-2 border-b border-surface-z2 text-sm">
        <span class="flex-1">{s.task}</span>
        <span class="ftr-mark" class:pass={s.ftr} class:fail={s.ftr === false}>{s.ftr === true ? '✓' : s.ftr === false ? '✗' : '—'}</span>
        <span class="opacity-50 text-xs">{new Date(s.startedAt).toLocaleDateString()}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .ftr-mark.pass { color: oklch(var(--color-success-z5) / 1); }
  .ftr-mark.fail { color: oklch(var(--color-primary-z5) / 1); }
  .session-row:last-child { border-bottom: none; }
</style>
