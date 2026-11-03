<script lang="ts">
  // The Code map's side panel (M17): what `inspect` says about the selection —
  // eyebrow, title, a line on what it is, its numbers, and what to look at.
  import type { Inspection, Tone } from './inspector';

  let { data, onopen }: { data: Inspection; onopen: (id: string) => void } = $props();

  const TONE: Record<Tone, string> = {
    ink: 'text-ink',
    danger: 'text-danger',
    warning: 'text-warning',
    success: 'text-success',
    faint: 'text-ink-faint',
  };
</script>

<aside class="flex flex-col gap-3 text-xs" data-testid="codemap-inspector">
  <div class="flex flex-col gap-1">
    <span class="uppercase tracking-wide text-ink-mute" data-testid="inspector-eyebrow">{data.eyebrow}</span>
    {#if data.title}
      <h4 class="m-0 font-mono text-sm text-ink break-all" data-testid="inspector-title">{data.title}</h4>
    {/if}
    {#if data.sub}<p class="m-0 text-ink-soft">{data.sub}</p>{/if}
  </div>
  {#if data.stats.length}
    <dl class="m-0 grid grid-cols-2 gap-x-3 gap-y-2">
      {#each data.stats as s (s.label)}
        <div class="flex flex-col" data-testid={`inspector-stat-${s.label}`}>
          <dt class="text-ink-mute">{s.label}</dt>
          <dd class={`m-0 font-mono text-sm ${TONE[s.tone]}`}>{s.value}</dd>
        </div>
      {/each}
    </dl>
  {/if}
  {#if data.open}
    <button class="self-start px-2 py-1 text-xs rounded border border-paper-edge bg-transparent text-ink cursor-pointer"
      onclick={() => onopen(data.open!)} data-testid="codemap-open">Open module → files</button>
  {/if}
  {#if data.list.length}
    <div class="flex flex-col gap-1">
      {#if data.listLabel}<span class="uppercase tracking-wide text-ink-mute">{data.listLabel}</span>{/if}
      <ul class="m-0 p-0 list-none flex flex-col gap-1 max-h-80 overflow-auto">
        {#each data.list as item, i (i)}
          <li class="flex items-baseline justify-between gap-2">
            <span class={`font-mono break-all ${TONE[item.tone]}`}>{item.text}</span>
            {#if item.value}<span class="font-mono text-ink-mute">{item.value}</span>{/if}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</aside>
