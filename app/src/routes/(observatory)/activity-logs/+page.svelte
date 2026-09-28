<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { EmptyState, Eyebrow } from '$lib/components';
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
    type LogFilters,
  } from './activity-logs.svelte.js';
  import LogRow from './LogRow.svelte';
  import ScheduledTaskRow from './ScheduledTaskRow.svelte';

  let { data } = $props();

  const view = new ActivityLogsView();

  // Fresh "x ago" baseline, recomputed whenever a new page loads. Kept a pure
  // derivation (not an $effect assigning state) — reading `data.rows` registers
  // the dependency so the baseline moves forward on every refetch.
  const now = $derived.by(() => {
    data.rows.length;
    return Date.now();
  });

  const sources = $derived(distinctSources(data.rows, data.filters.source));
  const modules = $derived(distinctModules(data.rows, data.filters.module));
  const rendered = $derived(searchRows(data.rows, view.search));

  // Server-side filters live in the URL; changing one refetches via the loader.
  // `keepFocus` + `noScroll` keep the control interaction smooth on refetch.
  function apply(patch: Partial<LogFilters>) {
    const next = { ...data.filters, ...patch };
    goto(`${page.url.pathname}${buildLogsQuery(next)}`, {
      invalidateAll: true,
      keepFocus: true,
      noScroll: true,
    });
  }
</script>

<div class="flex flex-col h-full" data-screen="activity-logs">
  <!-- Header -->
  <div class="flex items-center gap-5 pt-5 pb-4 px-6 border-b border-paper-edge">
    <span class="kanji text-3xl text-accent leading-none">診</span>
    <div class="flex-1 min-w-0">
      <p class="m-0 mb-1"><Eyebrow>Observatory · Logs</Eyebrow></p>
      <h1 class="display text-xl font-normal m-0">Activity logs</h1>
      <p class="text-sm text-ink-mute mt-1 mb-0 max-w-[640px] leading-relaxed">
        Daemon, cli, mcp and app activity as the machine recorded it — the surface to
        open when the numbers don't correlate or a signal isn't updating.
      </p>
    </div>
  </div>

  <!-- Background tasks (#96) — what the daemon's workers are, WHEN each is
       allowed to run, and when it last did. Schedule comes from
       `sensei.schedules` and is editable via PATCH /api/tasks/scheduled/{name}.
       Run-health (last_ok / error) stays null until the daemon records
       outcomes, so that cell still reads a muted em dash. -->
  {#if data.tasks.length > 0}
    <section class="border-b border-paper-edge" data-testid="scheduled-tasks">
      <div class="flex items-baseline gap-2 px-6 pt-4 pb-2">
        <h2 class="text-xs uppercase tracking-wider text-ink-soft m-0">Background tasks</h2>
        <span class="text-xs text-ink-faint">
          what the daemon is running, and when — run-health tracking is a follow-up
        </span>
      </div>
      <div
        class="grid grid-cols-[minmax(0,1fr)_96px_84px_128px_72px] items-center gap-3 px-6 py-1 bg-paper-soft border-y border-paper-edge text-xs uppercase tracking-wider text-ink-faint"
      >
        <span>worker</span>
        <span>last run</span>
        <span>interval</span>
        <span>schedule</span>
        <span>health</span>
      </div>
      {#each data.tasks as task (task.name)}
        <ScheduledTaskRow {task} {now} />
      {/each}
    </section>
  {/if}

  <!-- Controls -->
  <div class="flex items-center gap-3 flex-wrap px-6 py-3 border-b border-paper-edge">
    <!-- level (exact match — clearly labelled) -->
    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span class="uppercase tracking-wider">Level</span>
      <select
        class="font-mono text-xs bg-paper-soft text-ink border border-paper-edge rounded py-1 px-2 cursor-pointer"
        value={data.filters.level}
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
    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span class="uppercase tracking-wider">Source</span>
      <div class="max-w-[160px]" data-log-filter-source>
        <Select
          items={[{ label: 'All sources', value: '' }, ...sources.map((s) => ({ label: s, value: s }))]}
          value={data.filters.source}
          onchange={(v) => apply({ source: v as string })}
        />
      </div>
    </label>

    <!-- module -->
    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span class="uppercase tracking-wider">Module</span>
      <div class="max-w-[160px]" data-log-filter-module>
        <Select
          items={[{ label: 'All modules', value: '' }, ...modules.map((m) => ({ label: m, value: m }))]}
          value={data.filters.module}
          onchange={(v) => apply({ module: v as string })}
        />
      </div>
    </label>

    <span class="flex-1"></span>

    <!-- search (client-side, over message / module / source) -->
    <input
      type="search"
      class="text-xs bg-paper-soft text-ink border border-paper-edge rounded py-1 px-2 w-[180px] placeholder:text-ink-faint"
      placeholder="search message…"
      bind:value={view.search}
      data-log-search
    />

    <!-- since (time range) -->
    <div class="inline-flex bg-paper-mute rounded p-1" role="group" aria-label="Time range">
      {#each SINCE_OPTIONS as opt (opt.value)}
        {@const active = data.filters.since === opt.value}
        <button
          type="button"
          class="text-xs rounded py-2 px-3 cursor-pointer border-none"
          class:bg-paper={active}
          class:text-ink={active}
          class:font-semibold={active}
          class:bg-transparent={!active}
          class:text-ink-mute={!active}
          aria-pressed={active}
          onclick={() => apply({ since: opt.value })}
          data-log-since={opt.value}
        >
          {opt.label}
        </button>
      {/each}
    </div>

    <!-- limit -->
    <label class="flex items-center gap-2 text-xs text-ink-mute">
      <span class="uppercase tracking-wider">Limit</span>
      <div data-log-filter-limit>
        <Select
          items={[...LIMIT_OPTIONS]}
          value={data.filters.limit}
          onchange={(v) => apply({ limit: Number(v) })}
        />
      </div>
    </label>
  </div>

  <!-- Count / cap note -->
  <div class="flex items-center gap-2 px-6 py-2 border-b border-paper-edge text-xs text-ink-mute">
    <span class="font-mono" data-log-count>
      {#if view.search.trim()}
        showing {rendered.length} of {data.rows.length} loaded
      {:else}
        showing {data.rows.length} {data.rows.length === 1 ? 'log' : 'logs'}
      {/if}
    </span>
    {#if data.capped}
      <span class="text-warning" data-log-capped>
        · capped at {data.filters.limit} — narrow the range or raise the limit
      </span>
    {/if}
  </div>

  <!-- Column headers -->
  <div
    class="grid grid-cols-[86px_56px_84px_112px_minmax(0,1fr)_16px] items-center gap-3 px-6 py-2 bg-paper-soft border-b border-paper-edge text-xs uppercase tracking-wider text-ink-faint shrink-0"
  >
    <span>time</span>
    <span class="text-center">level</span>
    <span>source</span>
    <span>module</span>
    <span>message</span>
    <span></span>
  </div>

  <!-- Rows -->
  {#if data.rows.length === 0}
    <div class="p-6">
      <EmptyState
        kanji="診"
        title="no logs for these filters"
        description="Nothing matched the current level, source, module and time range. Widen the range or clear a filter."
      />
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto min-h-0">
      {#if rendered.length === 0}
        <p class="text-sm text-ink-mute px-6 py-6">No rows match “{view.search}”.</p>
      {:else}
        {#each rendered as row (row.id)}
          <LogRow
            {row}
            {now}
            expanded={view.expandedId === row.id}
            onToggle={() => view.toggle(row.id)}
          />
        {/each}
      {/if}
      <div class="h-7"></div>
    </div>
  {/if}
</div>
