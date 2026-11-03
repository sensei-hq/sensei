<script lang="ts">
  // The Code map's side panel (M17): what `inspect` says about the selection —
  // eyebrow, title, a line on what it is, its numbers as stat cards, its
  // actions, and what to look at. Under a focus it is the focus's own panel.
  import { StatCard, type StatCardTone } from '@rokkit/ui';
  import type { Inspection, Tone } from './inspector';

  let {
    data,
    onopen,
    onclear,
  }: {
    data: Inspection;
    onopen: (id: string) => void;
    /** Clear the focus — given only while something is focused. */
    onclear?: () => void;
  } = $props();

  const CARD: Record<Tone, StatCardTone | undefined> = { ink: undefined, danger: 'danger', warning: 'warning', success: 'success', faint: 'muted' };

  const TONE: Record<Tone, string> = {
    ink: 'text-ink',
    danger: 'text-danger',
    warning: 'text-warning',
    success: 'text-success',
    faint: 'text-ink-faint',
  };
</script>

<aside class="flex flex-col gap-3 text-sm" data-testid="codemap-inspector">
  <div class="flex flex-col gap-1">
    <span class="text-xs uppercase tracking-wide text-ink-mute" data-testid="inspector-eyebrow">{data.eyebrow}</span>
    {#if data.title}
      <h4 class="m-0 font-mono text-sm text-ink break-all" data-testid="inspector-title">{data.title}</h4>
    {/if}
    {#if data.sub}<p class="m-0 text-ink-soft">{data.sub}</p>{/if}
  </div>
  {#if data.stats.length}
    <div class="grid grid-cols-2 gap-2">
      {#each data.stats as st (st.label)}
        <StatCard label={st.label} value={st.value} tone={CARD[st.tone]} data-testid={`inspector-stat-${st.label}`} />
      {/each}
    </div>
  {/if}
  {#if data.open || onclear}
    <div class="flex flex-wrap gap-2">
      {#if data.open}
        <button class="px-3 py-1.5 text-sm rounded border border-paper-edge bg-transparent text-ink cursor-pointer inline-flex items-center gap-1.5"
          onclick={() => onopen(data.open!)} data-testid="codemap-open">
          <span class="i-solar-folder-open-bold-duotone" aria-hidden="true"></span>Open files</button>
      {/if}
      {#if onclear}
        <button class="px-3 py-1.5 text-sm rounded border border-paper-edge bg-transparent text-ink-mute cursor-pointer inline-flex items-center gap-1.5"
          onclick={onclear} data-testid="codemap-clear-focus">
          <span class="i-solar-close-circle-bold-duotone" aria-hidden="true"></span>Clear focus</button>
      {/if}
    </div>
  {/if}
  {#if data.list.length}
    <div class="flex flex-col gap-1">
      {#if data.listLabel}<span class="text-xs uppercase tracking-wide text-ink-mute">{data.listLabel}</span>{/if}
      <ul class="m-0 p-0 list-none flex flex-col gap-1 max-h-80 overflow-auto">
        {#each data.list as item, i (i)}
          <li class="flex items-baseline justify-between gap-2">
            <span class={`break-words ${TONE[item.tone]}`}>{item.text}</span>
            {#if item.value}<span class="font-mono text-ink-mute">{item.value}</span>{/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</aside>
