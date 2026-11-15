<script lang="ts">
  import { correctionsView, sessionFtrView } from '$lib/ftr';
  import { compactDuration, durationMinutes, timeOfDay } from '$lib/sessions-digest';
  interface Session {
    id: string;
    task: string;
    workspace: string;
    startedAt: string;
    completedAt?: string;
    ftr?: number | null;
    corrections?: number | null;
  }


  interface Props {
    sessions: Session[];
  }
  let { sessions }: Props = $props();

</script>

{#if sessions.length > 0}
  <section class="mt-6">
    <div class="flex items-baseline justify-between mb-3">
      <h2 class="display text-base font-normal m-0">Recent sessions</h2>
      <a href="/sessions" class="text-xs text-ink-mute hover:text-ink">all sessions →</a>
    </div>
    <div class="flex flex-col">
      {#each sessions as s (s.id)}
        {@const mins = durationMinutes(s.startedAt, s.completedAt ?? null)}
        {@const ftr = sessionFtrView(s.ftr)}
        <a
          data-session-row={s.id}
          href={`/sessions#${s.id}`}
          class="grid grid-cols-[8px_120px_1fr_auto_auto_auto] gap-4 py-3 px-1 items-center text-left border-b border-paper-edge hover:bg-paper-soft"
        >
          <span
            data-ftr-dot
            data-tone={ftr.tone}
            class="w-2 h-2 rounded-full {ftr.dot}"
          ></span>
          <span class="font-mono text-xs text-ink-mute truncate">{s.workspace}</span>
          <span class="text-sm text-ink-soft truncate">{s.task}</span>
          <span
            data-corrections={s.corrections ?? ''}
            class="font-mono text-xs min-w-[5rem] text-right {correctionsView(s.corrections).text}"
          >
            {correctionsView(s.corrections).label}
          </span>
          {#if mins !== null}
            <span data-duration class="font-mono text-xs text-ink-mute min-w-[3rem] text-right">
              {compactDuration(mins)}
            </span>
          {:else}
            <span></span>
          {/if}
          <span class="font-mono text-xs text-ink-mute">{timeOfDay(s.startedAt)}</span>
        </a>
      {/each}
    </div>
  </section>
{/if}
