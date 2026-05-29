<script lang="ts">
  interface Session {
    id: string;
    task: string;
    project: string;
    startedAt: string;
    completedAt?: string;
    ftr?: number | null;
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
      <a href="/sessions" class="text-xs text-surface-z6 hover:text-surface-z9">all sessions →</a>
    </div>
    <div class="flex flex-col">
      {#each sessions as s (s.id)}
        {@const mins = durationMinutes(s)}
        {@const isSuccess = (s.ftr ?? 0) >= 1}
        <a
          data-session-row={s.id}
          href={`/sessions#${s.id}`}
          class="grid grid-cols-[8px_120px_1fr_auto_auto] gap-4 py-3 px-1 items-center text-left border-b border-surface-z2 hover:bg-surface-z1"
        >
          <span
            data-ftr-dot
            data-tone={isSuccess ? 'success' : 'warn'}
            class="w-2 h-2 rounded-full"
            class:bg-success-z6={isSuccess}
            class:bg-warning-z6={!isSuccess}
          ></span>
          <span class="mono text-xs text-surface-z6 truncate">{s.project}</span>
          <span class="text-sm text-surface-z8 truncate">{s.task}</span>
          {#if mins !== null}
            <span data-duration class="mono text-xs text-surface-z6 min-w-[3rem] text-right">
              {formatDuration(mins)}
            </span>
          {:else}
            <span></span>
          {/if}
          <span class="mono text-xs text-surface-z5">{timeOfDay(s.startedAt)}</span>
        </a>
      {/each}
    </div>
  </section>
{/if}
