<script lang="ts">
  interface Session {
    id: string;
    task: string;
    project: string;
    startedAt: string;
    completedAt?: string;
    ftr?: number | null;
    corrections?: number | null;
  }

  // Compact "first-try" / "3× rework" tag per the mockup. `null` = daemon
  // hasn't reported yet (session in flight) → show a dash so the column
  // never collapses.
  function correctionsLabel(c: number | null | undefined): string {
    if (c == null) return '—';
    if (c === 0)   return 'first-try';
    return `${c}× rework`;
  }

  interface Props {
    sessions: Session[];
  }
  let { sessions }: Props = $props();

  function durationMinutes(s: Session): number | null {
    if (!s.completedAt) return null;
    const start = Date.parse(s.startedAt);
    const end   = Date.parse(s.completedAt);
    if (Number.isNaN(start) || Number.isNaN(end) || end <= start) return null;
    return Math.round((end - start) / 60_000);
  }

  function formatDuration(mins: number): string {
    if (mins < 60) return `${mins}m`;
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return m === 0 ? `${h}h` : `${h}h ${m}m`;
  }

  function timeOfDay(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false });
  }
</script>

{#if sessions.length > 0}
  <section class="mt-6">
    <div class="flex items-baseline justify-between mb-3">
      <h2 class="display text-base font-normal m-0">Recent sessions</h2>
      <a href="/sessions" class="text-xs text-ink-soft hover:text-ink">all sessions →</a>
    </div>
    <div class="flex flex-col">
      {#each sessions as s (s.id)}
        {@const mins = durationMinutes(s)}
        {@const isSuccess = (s.ftr ?? 0) >= 1}
        <a
          data-session-row={s.id}
          href={`/sessions#${s.id}`}
          class="grid grid-cols-[8px_120px_1fr_auto_auto_auto] gap-4 py-3 px-1 items-center text-left border-b border-paper-edge hover:bg-paper-soft"
        >
          <span
            data-ftr-dot
            data-tone={isSuccess ? 'success' : 'warn'}
            class="w-2 h-2 rounded-full"
            class:bg-success={isSuccess}
            class:bg-warning={!isSuccess}
          ></span>
          <span class="font-mono text-xs text-ink-soft truncate">{s.project}</span>
          <span class="text-sm text-ink-mute truncate">{s.task}</span>
          <span
            data-corrections={s.corrections ?? ''}
            class="font-mono text-xs min-w-[5rem] text-right"
            class:text-success={s.corrections === 0}
            class:text-warning={typeof s.corrections === 'number' && s.corrections > 0}
            class:text-ink-soft={s.corrections == null}
          >
            {correctionsLabel(s.corrections)}
          </span>
          {#if mins !== null}
            <span data-duration class="font-mono text-xs text-ink-soft min-w-[3rem] text-right">
              {formatDuration(mins)}
            </span>
          {:else}
            <span></span>
          {/if}
          <span class="font-mono text-xs text-ink-soft">{timeOfDay(s.startedAt)}</span>
        </a>
      {/each}
    </div>
  </section>
{/if}
