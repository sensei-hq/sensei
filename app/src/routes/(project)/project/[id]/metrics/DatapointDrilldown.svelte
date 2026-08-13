<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { Kanji, Eyebrow } from '$lib/components';
    import {
        formatDayLabel,
        sessionOneLiner,
        shortSessionId,
        type MetricSeriesPoint,
    } from '$lib/metrics/metric-view.js';
    import { DatapointDrilldownState } from './datapoint-drilldown.svelte.js';

    // The day-scoped datapoint drill-down: for a selected day it shows that day's
    // explainer (why the value reads the way it does) and that day's measurable
    // sessions, each with its per-session observation. The controller owns the
    // day state + the fetch lifecycle (props + controller state in, markup out).
    // The controller is injectable so the spec drives every state with a
    // fixture-backed api; the default is a live controller over the daemon.
    let {
        series,
        projectId,
        metricKey,
        seriesError = null,
        controller = new DatapointDrilldownState(senseiApi(appState.port)),
    }: {
        series: MetricSeriesPoint[];
        projectId: string;
        metricKey: string;
        /** The loader's daily-series fetch failure, if any — surfaced as an
         *  error state instead of masking it as honest-empty. */
        seriesError?: string | null;
        controller?: DatapointDrilldownState;
    } = $props();

    onMount(() => {
        void controller.init(series, projectId, metricKey, seriesError);
    });
    onDestroy(() => controller.dispose());
</script>

<section
    data-component="datapoint-drilldown"
    data-status={controller.status}
    class="flex flex-col gap-3"
>
    <div class="flex items-center gap-2">
        <Kanji char="証" size="sm" tone="accent" />
        <Eyebrow>Behind this datapoint</Eyebrow>
    </div>

    {#if controller.days.length}
        <div
            data-drilldown-day-selector
            class="flex flex-wrap gap-1 p-1 bg-paper border border-paper-edge rounded-md self-start"
        >
            {#each controller.days as day (day)}
                {@const active = day === controller.selectedDay}
                <button
                    type="button"
                    data-day={day}
                    data-active={active}
                    onclick={() => controller.select(day)}
                    class="px-3 py-1 rounded-md text-xs transition-colors duration-fast {active
                        ? 'bg-ink text-paper'
                        : 'text-ink-mute hover:text-ink'}"
                >{formatDayLabel(day)}</button>
            {/each}
        </div>
    {/if}

    {#if controller.explainer}
        <p
            data-drilldown-explainer
            class="text-sm text-ink-soft leading-relaxed text-pretty"
        >{controller.explainer}</p>
    {/if}

    {#if controller.status === 'loading'}
        <p data-drilldown-loading class="text-sm text-ink-faint">Reading that day’s sessions…</p>
    {:else if controller.status === 'unavailable'}
        <p data-drilldown-unavailable class="text-sm text-ink-faint">
            This drill-down isn’t available on the connected daemon yet.
        </p>
    {:else if controller.status === 'error'}
        <p data-drilldown-error class="text-sm text-ink-mute">
            Couldn’t load the drill-down — {controller.errorMessage}
        </p>
    {:else if controller.status === 'empty'}
        <p data-drilldown-empty class="text-sm text-ink-faint">No measurable sessions that day.</p>
    {:else}
        <div class="flex flex-col">
            {#each controller.sessions as s (s.client_session_id ?? s.started_at)}
                <div
                    data-session={s.client_session_id}
                    class="flex flex-col gap-1 py-3 border-t border-paper-edge"
                >
                    <div class="flex items-baseline justify-between gap-3">
                        <span class="font-mono text-xs text-ink-mute truncate"
                            >{shortSessionId(s.client_session_id)}</span
                        >
                        <span data-session-meta class="text-xs text-ink-faint shrink-0"
                            >{sessionOneLiner(s)}</span
                        >
                    </div>
                    <div class="text-sm text-ink text-pretty">{s.task}</div>
                    {#if s.summary && s.summary.trim()}
                        <div class="text-sm text-ink-soft text-pretty">{s.summary}</div>
                    {/if}
                    <div
                        data-session-observation
                        class="flex flex-col gap-1 border-l border-accent-soft pl-3 mt-1"
                    >
                        <span class="text-xs text-ink-faint">{s.observation.title}</span>
                        <span class="text-sm text-ink-soft text-pretty">{s.observation.detail}</span>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</section>
