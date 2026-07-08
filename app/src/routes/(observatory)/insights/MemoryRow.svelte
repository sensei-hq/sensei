<script lang="ts">
  import type { MemoryRowVm } from './insights-board.svelte.js';

  let {
    memory,
    variant = 'settled',
  }: {
    memory: MemoryRowVm;
    /** 'settled' = in-force reference row with a strength bar;
     *  'proposed' = a memory awaiting a keep/retire decision (Soon). */
    variant?: 'settled' | 'proposed';
  } = $props();

  // Strength bar fill width (0–1 fraction → %). Geometry only.
  const fillWidth = $derived(`${Math.round(memory.strength * 100)}%`);
  const full = $derived(memory.strength >= 1);
</script>

{#if variant === 'proposed'}
  <!-- A memory contested / awaiting a keep-or-retire decision. Display-only. -->
  <article
    data-component="memory-row"
    data-variant="proposed"
    data-memory-id={memory.id}
    class="rounded bg-warning-soft border border-transparent py-2 px-3"
  >
    <div class="text-xs uppercase tracking-wider text-warning mb-1">proposed</div>
    <p class="text-sm text-ink m-0 leading-snug">{memory.title}</p>
  </article>
{:else}
  <!-- Battle-tested reference row, sorted by strength. Display-only shelf. -->
  <article
    data-component="memory-row"
    data-variant="settled"
    data-memory-id={memory.id}
    class="flex items-baseline gap-2 py-1"
  >
    <div
      class="mt-1 h-[3px] w-6 shrink-0 rounded-full bg-paper-mute overflow-hidden"
      title={`strength ${fillWidth}`}
    >
      <div
        class="h-full rounded-full"
        class:bg-success={full}
        class:bg-ink-soft={!full}
        style={`width: ${fillWidth}`}
        data-strength-fill
      ></div>
    </div>
    <div class="flex-1 min-w-0">
      <p class="text-sm text-ink m-0 leading-snug truncate">{memory.title}</p>
      <span class="font-mono text-xs text-ink-mute">{memory.scope}</span>
    </div>
  </article>
{/if}
