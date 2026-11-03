<script lang="ts">
  // The Complexity lens (user, 2026-11-02): a tree — module, folder, file — each
  // row carrying its functions, how many are over cyclomatic 10 and the highest
  // cyclomatic under it, led by the workspace's overall reading and its
  // maintainability grade. A rokkit Tree; one snippet draws a row's numbers
  // whether it opens or not. A module row opens the module.
  import { Tree } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import type { ComplexityNode, ComplexityTree } from './types';

  let { data, zoom = 1, onfocus }: { data: ComplexityTree | null; zoom?: number; onfocus: (id: string) => void } = $props();

  const pct = (complex: number, functions: number) => (functions ? Math.round((complex / functions) * 100) : 0);
  /** The share's tone: over 20% red, over 10% amber — the lens's tiers. */
  const tone = (n: { complex: number; functions: number }) => {
    const share = n.functions ? n.complex / n.functions : 0;
    return share > 0.2 ? 'text-danger' : share > 0.1 ? 'text-warning' : 'text-ink-mute';
  };
  const items = $derived(
    (data?.modules ?? []).map(function toItem(n: ComplexityNode, i: number): Record<string, unknown> {
      return {
        label: n.label,
        value: n.id,
        node: n,
        // The most complex module opens on arrival; the rest stay closed.
        expanded: n.kind === 'module' && i === 0,
        children: n.children.length ? n.children.map(toItem) : undefined,
      };
    }),
  );
  const GRADE_TONE: Record<string, string> = {
    A: 'bg-success-soft text-on-success-soft', B: 'bg-success-soft text-on-success-soft',
    C: 'bg-warning-soft text-on-warning-soft', D: 'bg-danger-soft text-on-danger-soft', F: 'bg-danger-soft text-on-danger-soft',
  };
</script>

{#snippet row(proxy: ProxyItem)}
  {@const n = (proxy.original as { node: ComplexityNode }).node}
  <span class="flex items-baseline gap-3 w-full min-w-0" data-complexity-row={n.id}>
    <span class="truncate" class:font-mono={n.kind !== 'module'} class:font-medium={n.kind === 'module'} title={n.id}>{n.label}</span>
    <span class="ml-auto shrink-0 tabular-nums text-xs {tone(n)}">{n.complex} of {n.functions} · {pct(n.complex, n.functions)}%</span>
    <span class="shrink-0 tabular-nums text-xs text-ink-faint w-16 text-right">max {n.maxCyclo}</span>
    {#if n.kind === 'module'}
      <button class="shrink-0 p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:text-ink"
        aria-label={`Focus ${n.id}`} title={`Focus ${n.id}`} onclick={(e) => { e.stopPropagation(); onfocus(n.id); }}>↗</button>
    {/if}
  </span>
{/snippet}

<!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="w-full overflow-auto flex flex-col gap-4" tabindex="0" role="region" aria-label="Complexity by module and folder, scrollable"
  data-testid="codemap-complexity" style:font-size={`${Math.round(13 * zoom)}px`}>
  {#if !data}
    <p class="m-0 text-ink-faint">Reading the measured complexity…</p>
  {:else if !data.total.files}
    <p class="m-0 text-ink-mute">Not measured yet: complexity is measured by local qlty after a scan.</p>
  {:else}
    <div class="flex flex-wrap items-center gap-x-6 gap-y-2 p-4 rounded-lg border border-paper-edge bg-paper-soft" data-testid="complexity-overall">
      <span class="text-sm">
        <span class="tabular-nums {tone(data.total)}">{data.total.complex} of {data.total.functions} functions over cyclomatic 10</span>
        <span class="text-ink-mute"> · {pct(data.total.complex, data.total.functions)}%</span>
      </span>
      <span class="text-sm text-ink-mute">highest <span class="tabular-nums text-ink">{data.total.maxCyclo}</span></span>
      <span class="text-sm text-ink-mute"><span class="tabular-nums text-ink">{data.total.files}</span> files measured</span>
      <span class="flex items-center gap-2 text-sm text-ink-mute">
        Maintainability
        {#if data.maintainability}
          {#if data.maintainability.grade}
            <span class="rounded px-1.5 py-0.5 text-xs font-medium {GRADE_TONE[data.maintainability.grade] ?? ''}" data-testid="complexity-grade">{data.maintainability.grade}</span>
          {/if}
          <span class="text-ink">{data.maintainability.label}</span>
        {:else}
          <span class="text-ink-faint">not computed</span>
        {/if}
      </span>
    </div>
    <Tree {items} itemContent={row} groupContent={row} />
  {/if}
</div>
