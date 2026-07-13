<script lang="ts">
  import type { LogRow } from '$lib/types.js';
  import {
    levelTone,
    moduleOf,
    relativeTime,
    absoluteTime,
    payloadSections,
  } from './activity-logs.svelte.js';

  interface Props {
    row: LogRow;
    /** `Date.now()` at render — kept a prop so relative time stays pure/testable. */
    now: number;
    expanded: boolean;
    onToggle: () => void;
  }
  let { row, now, expanded, onToggle }: Props = $props();

  const tone = $derived(levelTone(row.level));
  const mod = $derived(moduleOf(row));
  const sections = $derived(payloadSections(row));
  const canExpand = $derived(sections.length > 0);
</script>

<div class="border-b border-paper-edge" data-component="log-row" data-log-level={row.level}>
  <button
    type="button"
    class="grid grid-cols-[86px_56px_84px_112px_minmax(0,1fr)_16px] items-center gap-3 w-full text-left py-2 px-6 bg-transparent border-none"
    class:cursor-pointer={canExpand}
    class:cursor-default={!canExpand}
    disabled={!canExpand}
    aria-expanded={canExpand ? expanded : undefined}
    onclick={onToggle}
    data-log-row-header
  >
    <span class="font-mono text-xs text-ink-mute truncate" title={absoluteTime(row.logged_at)}>
      {relativeTime(now, row.logged_at)}
    </span>

    <span
      class="text-xs uppercase tracking-wider text-center rounded py-[2px] px-1 {tone.text} {tone.bg}"
      data-log-level-chip
    >
      {row.level}
    </span>

    <span class="font-mono text-xs text-ink-mute truncate">
      {row.source ?? '—'}
    </span>

    <span class="font-mono text-xs text-ink-soft truncate">
      {mod ?? '—'}
    </span>

    <span class="text-sm text-ink truncate" title={row.message}>
      {row.message}
    </span>

    <span class="text-xs text-ink-faint text-center leading-none">
      {#if canExpand}{expanded ? '▾' : '▸'}{/if}
    </span>
  </button>

  {#if expanded && canExpand}
    <div class="bg-paper-mute rounded border border-paper-edge mb-3 ml-6 mr-6 py-3 px-3" data-log-payload>
      {#each sections as section (section.label)}
        <div class="mb-3 last:mb-0" data-log-section={section.label}>
          <div
            class="text-xs uppercase tracking-wider mb-1"
            class:text-error={section.label === 'error'}
            class:text-ink-faint={section.label !== 'error'}
          >
            {section.label}
          </div>
          <pre
            class="font-mono text-xs leading-relaxed whitespace-pre-wrap break-words m-0"
            class:text-error={section.label === 'error'}
            class:text-ink-soft={section.label !== 'error'}>{section.json}</pre>
        </div>
      {/each}
    </div>
  {/if}
</div>
