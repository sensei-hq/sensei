<script lang="ts">
  import { Notice, EmptyState } from '@sensei/ui';
  import { ScreenShell, type FilterState } from '@rokkit/app';
  import { untrack } from 'svelte';
  import { clock } from '$lib/clock.svelte.js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { useGlyphMode } from '$lib/glyphs';
  import { railIcon } from '../observatory-nav';
  import { Select } from '@rokkit/ui';
  import {
    LEVELS,
    SINCE_OPTIONS,
    LIMIT_OPTIONS,
    buildLogsQuery,
    searchRows,
    distinctSources,
    distinctModules,
    ActivityLogsView,
    LogsScreen,
    type LogFilters,
  } from './activity-logs.svelte.js';
  import LogRow from './LogRow.svelte';
  import ScheduledTaskRow from './ScheduledTaskRow.svelte';
  import { TaskLog } from './scheduled-tasks.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';

  let { data } = $props();

  // The screen's own state, as settings/extensions keeps its own: the loader's
  // answer, and whatever the notices' Retry read since. A navigation replaces it
  // whole — the server filters live in the URL, so the loader has the last word;
  // a retry writes back only its half, so a failed log re-read cannot take the
  // workers away with it (or the reverse).
  const screen = new LogsScreen(untrack(() => data));
  $effect(() => screen.setData(data));

  const shown = $derived(screen.shown);

  const view = new ActivityLogsView();

  // The shell owns the screen's one search field (contract §6); its `q` is what
  // narrows the loaded rows, and it is client-only — the filters below are the
  // server's.
  let filter = $state<FilterState>({ values: {}, stat: null });
  const q = $derived(String(filter.values.q ?? ''));
  const rendered = $derived(searchRows(shown.rows, q));

  // One log tail per worker, opened from its row (#96).
  const api = senseiApi(appState.port);
  const logs = new Map<string, TaskLog>();
  function logFor(name: string): TaskLog {
    let l = logs.get(name);
    if (!l) {
      l = new TaskLog(() => api.tryGetScheduledLog(name));
      logs.set(name, l);
    }
    return l;
  }

  // "x ago", "running" and "next in …" read the app's ticking clock, so they
  // move while the screen sits open (audit 2026-10-28, item 14).
  $effect(() => {
    clock.start();
    return () => clock.stop();
  });
  const now = $derived(clock.now);

  // The shell draws the mark as kanji or, in icon mode, as its icon.
  const glyphMode = useGlyphMode();
  const glyphs = $derived(glyphMode() === 'icon' ? 'icon' : 'text');

  const sources = $derived(distinctSources(shown.rows, shown.filters.source));
  const modules = $derived(distinctModules(shown.rows, shown.filters.module));

  // Server-side filters live in the URL; changing one refetches via the loader.
  // `keepFocus` + `noScroll` keep the control interaction smooth on refetch.
  function apply(patch: Partial<LogFilters>) {
    const next = { ...shown.filters, ...patch };
    goto(`${page.url.pathname}${buildLogsQuery(next)}`, {
      invalidateAll: true,
      keepFocus: true,
      noScroll: true,
    });
  }

  // The sidebar is a panel, not a spare column: a daemon with no workers and no
  // tasks error has nothing to put there, so it is not passed at all (an empty
  // 17.5rem aside is a blank with no next step).
  const hasWorkers = $derived(Boolean(shown.tasksNotice) || shown.tasks.length > 0);

  // A Notice's next step is Retry, and it is the only thing that re-reads: the
  // remedy already says so, and a dead end is worse than a second request.
  const retry = (onclick: () => void, busy: boolean) => ({
    label: 'Retry',
    onclick,
    busy,
    name: 'retry',
  });
</script>

<!-- The shell's filter band: the server-side controls, one search beside them
     (contract §6). -->
{#snippet filtersPanel()}
  <div class="flex items-center gap-3 flex-wrap">
    <!-- level (exact match — clearly labelled) -->
    <label class="flex items-center gap-2 text-xs text-ink-soft">
      <span class="uppercase tracking-wider">Level</span>
      <select
        class="font-mono text-xs bg-paper-soft text-ink border border-paper-edge rounded py-1 px-2 cursor-pointer"
        value={shown.filters.level}
        title="exact level match"
        onchange={(e) => apply({ level: e.currentTarget.value })}
        data-log-filter-level
      >
        <option value="">All levels</option>
        {#each LEVELS as level (level)}
          <option value={level}>{level}</option>
        {/each}
      </select>
    </label>

    <!-- source -->
    <label class="flex items-center gap-2 text-xs text-ink-soft">
      <span class="uppercase tracking-wider">Source</span>
      <div class="max-w-[160px]" data-log-filter-source>
        <Select
          items={[{ label: 'All sources', value: '' }, ...sources.map((s) => ({ label: s, value: s }))]}
          value={shown.filters.source}
          onchange={(v) => apply({ source: v as string })}
        />
      </div>
    </label>

    <!-- module -->
    <label class="flex items-center gap-2 text-xs text-ink-soft">
      <span class="uppercase tracking-wider">Module</span>
      <div class="max-w-[160px]" data-log-filter-module>
        <Select
          items={[{ label: 'All modules', value: '' }, ...modules.map((m) => ({ label: m, value: m }))]}
          value={shown.filters.module}
          onchange={(v) => apply({ module: v as string })}
        />
      </div>
    </label>

    <span class="flex-1"></span>

    <!-- since (time range) -->
    <div class="inline-flex bg-paper-mute rounded p-1" role="group" aria-label="Time range">
      {#each SINCE_OPTIONS as opt (opt.value)}
        {@const active = shown.filters.since === opt.value}
        <button
          type="button"
          class="text-xs rounded py-2 px-3 cursor-pointer border-none"
          class:bg-paper={active}
          class:text-ink={active}
          class:font-semibold={active}
          class:bg-transparent={!active}
          class:text-ink-soft={!active}
          aria-pressed={active}
          onclick={() => apply({ since: opt.value })}
          data-log-since={opt.value}
        >
          {opt.label}
        </button>
      {/each}
    </div>

    <!-- limit -->
    <label class="flex items-center gap-2 text-xs text-ink-soft">
      <span class="uppercase tracking-wider">Limit</span>
      <div data-log-filter-limit>
        <Select
          items={[...LIMIT_OPTIONS]}
          value={shown.filters.limit}
          onchange={(v) => apply({ limit: Number(v) })}
        />
      </div>
    </label>
  </div>
{/snippet}

<!-- Background tasks (#96) — what the daemon's workers are, WHEN each is
     allowed to run, when it last did and how it went, when it runs next and how
     long a pass takes. Schedule comes from `sensei.schedules` and is editable
     via PATCH /api/tasks/scheduled/{name}; a failing worker shows its last error
     under its name. Their own scroller, so the rows can scroll without them
     (#293); stacked, because the sidebar's 17.5rem cannot hold seven columns. -->
{#snippet workersPanel()}
  {#if shown.tasksNotice}
    <div class="px-4 py-4">
      <Notice
        size="panel"
        {...shown.tasksNotice}
        primary={retry(() => void screen.retryTasks(api), screen.readingTasks)}
        data-tasks-error
      />
    </div>
  {:else if shown.tasks.length > 0}
    <section data-testid="scheduled-tasks">
      <div class="flex flex-col gap-1 px-4 pt-4 pb-2">
        <h2 class="text-xs uppercase tracking-wider text-ink-mute m-0">Background tasks</h2>
        <span class="text-xs text-ink-faint">
          what the daemon is running, and when — run-health tracking is a follow-up
        </span>
      </div>
      {#each shown.tasks as task (task.name)}
        <ScheduledTaskRow {task} {now} log={logFor(task.name)} layout="stack" />
      {/each}
    </section>
  {/if}
{/snippet}

<!-- #293: the header, the workers, the filters and the count and column
     headings stay in place; the rows scroll. D87: rokkit's shell draws that
     layout — the title bar and the band are the header, the body and the right
     sidebar are the two scrollers. Raw shell for now: glyphs from the app's
     preference, no panel persistence until the `@sensei/ui` wrapper lands.

     The outer div carries `data-screen="activity-logs"` because the shell takes
     no rest props and every existing e2e opens this screen by that hook
     (contract Addendum 2). `h-full` keeps the height chain the old `Screen`
     gave it. Both go away when the shell passes attributes through (rokkit). -->
<div data-screen="activity-logs" class="h-full min-h-0">
  <ScreenShell
    viewId="observatory.logs"
    mark={{ kanji: '診', icon: railIcon('/activity-logs') }}
    title="Activity logs"
    context="Observatory · Logs"
    description="Daemon, cli, mcp and app activity as the machine recorded it — the surface to open when the numbers don't correlate or a signal isn't updating."
    {glyphs}
    search={{ placeholder: 'search message…' }}
    bind:filter
    filters={filtersPanel}
    sidebar={hasWorkers ? workersPanel : undefined}
  >
    {#if shown.rowsNotice}
      <!-- A read that failed has no count: "showing 0 logs" beside an error says
           the log is empty when nobody could read it (D83). -->
      <div class="px-6 py-6">
        <Notice
          size="panel"
          {...shown.rowsNotice}
          primary={retry(() => void screen.retryLogs(api), screen.readingLogs)}
          data-logs-error
        />
      </div>
    {:else}
      <!-- Count, cap note and column headings pin together at the top of the
           body's scroller (#293): the count is a claim about the rows right
           below it, so it does not scroll away from them. -->
      <div class="sticky top-0 z-10 bg-paper-soft">
        <div class="flex items-center gap-2 px-6 py-2 border-b border-paper-edge text-xs text-ink-soft">
          <span class="font-mono" data-log-count>
            {#if q.trim()}
              showing {rendered.length} of {shown.rows.length} loaded
            {:else}
              showing {shown.rows.length} {shown.rows.length === 1 ? 'log' : 'logs'}
            {/if}
          </span>
          {#if shown.capped}
            <span class="text-warning" data-log-capped>
              · capped at {shown.filters.limit} — narrow the range or raise the limit
            </span>
          {/if}
        </div>

        <div
          class="grid grid-cols-[86px_56px_84px_112px_minmax(0,1fr)_16px] items-center gap-3 px-6 py-2 border-b border-paper-edge text-xs uppercase tracking-wider text-ink-faint"
        >
          <span>time</span>
          <span class="text-center">level</span>
          <span>source</span>
          <span>module</span>
          <span>message</span>
          <span></span>
        </div>
      </div>

      {#if shown.rows.length === 0}
        <div class="p-6">
          <EmptyState size="screen"
            kanji="診"
            title="no logs for these filters"
            description="Nothing matched the current level, source, module and time range. Widen the range or clear a filter."
          />
        </div>
      {:else if rendered.length === 0}
        <p class="text-sm text-ink-soft px-6 py-6">No rows match “{q}”.</p>
      {:else}
        {#each rendered as row (row.id)}
          <LogRow
            {row}
            {now}
            expanded={view.expandedId === row.id}
            onToggle={() => view.toggle(row.id)}
          />
        {/each}
        <div class="h-7"></div>
      {/if}
    {/if}
  </ScreenShell>
</div>
