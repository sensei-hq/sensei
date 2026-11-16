<script lang="ts">
  // Relay · a past run, read-only (CONTRACT §1i; #424, the UI half of #393): the banner
  // (archived when its last task was done; deleted 30 days on, by a daily job), the
  // facts, who was on it, and what it did — each task opening its task view. Nothing to
  // start, cancel or answer. Shared by both windows: the route decides what was read and
  // the base of every link.
  import { invalidateAll } from '$app/navigation';
  import { List, Pill } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import { Notice, ShellScreen } from '@sensei/ui';
  import { toArchivedRun, type ApiError, type RelayArchivedRunView, type RelayRunDetail } from '@sensei/state';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';
  import SectionHeading from './SectionHeading.svelte';
  import { NOW_ROW } from './primitives.js';

  let {
    detail,
    error,
    viewId,
    base,
  }: { detail: RelayRunDetail | null; error: ApiError | null; viewId: string; base: string } = $props();

  const mode = useGlyphMode();
  const run = $derived(detail ? toArchivedRun(detail, { base, at: Date.now() }) : null);
  type Task = RelayArchivedRunView['tasks'][number];
</script>

<ShellScreen
  data-screen="relay"
  {viewId}
  glyphs={mode()}
  mark={{ kanji: '継', icon: KANJI_ICONS['継'] }}
  title={run ? `${run.title} · ${run.name}` : 'Relay'}
  context="past run"
  search={false}
>
  <div class="flex flex-col gap-4 p-4">
    <a data-back href={base} class="w-fit text-sm text-accent no-underline hover:underline"><span aria-hidden="true">‹&nbsp;</span>Relay</a>
    {#if error?.status === 404}
      <Notice
        data-relay-not-found
        tone="refusal"
        size="panel"
        title="This run isn't here."
        detail="It may have been deleted, 30 days after it ended, or belong to another person."
        code="http_404"
      />
    {:else if error}
      <Notice
        data-relay-error
        tone="fault"
        size="panel"
        title="Couldn't read this run"
        detail={error.message}
        code={error.status === 0 ? 'daemon_unreachable' : `http_${error.status}`}
        primary={{ label: 'Retry', name: 'retry', onclick: () => void invalidateAll() }}
      />
    {:else if run}
      <section data-archived-run aria-label={run.title} class="flex flex-col gap-4">
        <header class="flex flex-wrap items-start gap-x-6 gap-y-3 py-3.5 px-4 bg-paper-soft border border-solid border-paper-edge rounded-xl">
          <div class="flex-[1_1_320px] min-w-0 flex flex-col gap-1">
            <h3 data-run-title class="m-0 flex flex-wrap items-center gap-2 display text-lg font-medium">
              <span>{run.title}</span>
              <Pill value={run.state.label} tone={run.state.tone} size="sm" />
            </h3>
            <p class="m-0 text-sm text-ink-soft">{run.name}</p>
            <p data-run-banner data-warning={run.banner.warning} class={['m-0 text-xs', run.banner.warning ? 'text-warning font-semibold' : 'text-ink-mute']}>
              {run.banner.text}
            </p>
          </div>
          <dl class="m-0 flex gap-4.5">
            {#each run.facts as fact (fact.label)}
              <div data-run-fact class="flex flex-col items-end">
                <dt class="text-xs uppercase tracking-[0.12em] text-ink-mute">{fact.label}</dt>
                <dd class="m-0 display text-lg tabular-nums">{fact.value}</dd>
              </div>
            {/each}
          </dl>
        </header>

        <section aria-label="Who was on it" class="flex flex-col gap-1.5">
          <SectionHeading label="Who was on it" count={run.members.length} />
          <ul class="m-0 p-0 list-none flex flex-wrap gap-2">
            {#each run.members as member (member.id)}
              <li data-run-member={member.laneCode} class="flex items-center gap-2 rounded-lg border border-solid border-paper-edge bg-paper px-3 py-1.5 text-sm">
                <strong class="font-mono font-semibold">{member.codename}</strong>
                <span class="text-xs text-ink-mute">{member.role} · {member.family} · {member.model}</span>
              </li>
            {/each}
          </ul>
        </section>

        {#if run.tasks.length}
          <section aria-label="What it did" class="flex flex-col gap-1.5">
            <SectionHeading label="What it did" count={run.tasks.length} />
            <List items={run.tasks} fields={{ value: 'id' }} selectable={false} variant="card" label="What it did">
              {#snippet itemContent(proxy: ProxyItem)}
                {@const task = proxy.original as Task}
                <span data-run-task={task.id} class={NOW_ROW}>
                  <span class="min-w-0 flex-[1_1_240px] text-base md:truncate">{task.title}</span>
                  <span class="text-xs text-ink-mute">{task.by}</span>
                  <Pill value={task.state.label} tone={task.state.tone} size="sm" />
                </span>
              {/snippet}
            </List>
          </section>
        {/if}
      </section>
    {/if}
  </div>
</ShellScreen>
