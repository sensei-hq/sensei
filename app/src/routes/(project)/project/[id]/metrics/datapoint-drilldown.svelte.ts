// Metric detail · datapoint drill-down — the controller behind
// `DatapointDrilldown.svelte`, so the component stays a pure template (props +
// controller state in, markup out — mirrors `local-models.svelte.ts`).
//
// It answers "what's the evidence behind this datapoint" for a selected day:
// the day's EXPLAINER (from the daily series point, pure) and the day's
// SESSIONS with their per-session observation (fetched from the daemon). The
// day selector switches days; the default is the most recent day WITH data.
//
// Governance (no fabrication): a failed/absent fetch surfaces as an explicit
// `unavailable` (404 — the endpoint predates the connected daemon) or `error`
// STATE; a day with no measurable session is honest-`empty`. It NEVER invents a
// session, a day, or a zero.

import type { SenseiApi } from '$lib/api.js';
import {
    mostRecentDayWithValue,
    recentDaysWithData,
    explainerForDay,
    type MetricSeriesPoint,
    type DrilldownSession,
} from '$lib/metrics/metric-view.js';

/** The drill-down's render state:
 *  - `loading`     — a day's sessions are in flight
 *  - `ready`       — the day has ≥1 measurable session (rendered)
 *  - `empty`       — the fetch succeeded with 0 sessions, OR the series has no
 *                    day with data at all (honest-empty)
 *  - `unavailable` — the endpoint 404'd (a daemon that predates it) — a plain
 *                    "not available yet" notice, never a fabricated session
 *  - `error`       — any other fetch failure — the daemon's message is surfaced */
export type DrilldownStatus = 'loading' | 'ready' | 'empty' | 'unavailable' | 'error';

/** How many recent days-with-data the selector offers. */
export const DRILLDOWN_DAY_LIMIT = 7;

export class DatapointDrilldownState {
    /** The currently selected day (`YYYY-MM-DD`), or null when the series has no
     *  day with data. */
    selectedDay = $state<string | null>(null);
    status = $state<DrilldownStatus>('empty');
    sessions = $state<DrilldownSession[]>([]);
    /** The daemon's message for the `error` status (empty otherwise). */
    errorMessage = $state('');

    #api: SenseiApi;
    #series = $state<MetricSeriesPoint[]>([]);
    #projectId = '';
    #metricKey = '';
    /** Monotonic request token — a slow response for a superseded day is dropped
     *  (the latest selection always wins), so day-switching can't render stale
     *  sessions under the wrong day. */
    #token = 0;
    #disposed = false;
    /** The in-flight load, exposed via `settled()` for deterministic tests. */
    #inflight: Promise<void> | null = null;

    constructor(api: SenseiApi) {
        this.#api = api;
    }

    /** The recent days-with-data for the selector (ascending, newest last). */
    get days(): string[] {
        return recentDaysWithData(this.#series, DRILLDOWN_DAY_LIMIT);
    }

    /** The selected day's explainer from the series (null when absent — no
     *  fabricated copy). Reactive on `selectedDay`. */
    get explainer(): string | null {
        return explainerForDay(this.#series, this.selectedDay);
    }

    /** Adopt the daily series + scope, default the day to the most recent with
     *  data, and load it. Called from the component's `onMount` (the parent keys
     *  the component on the metric, so a new metric remounts and re-inits).
     *
     *  `seriesError` is the loader's daily-series FETCH failure (if any): when
     *  set, the drill-down's own input never arrived, so we surface it as an
     *  `error` state rather than defaulting an empty series to honest-`empty`
     *  (which would mask the failure — fail-closed governance). */
    init(
        series: MetricSeriesPoint[],
        projectId: string,
        metricKey: string,
        seriesError: string | null = null,
    ): Promise<void> {
        this.#series = series;
        this.#projectId = projectId;
        this.#metricKey = metricKey;
        if (seriesError) {
            this.selectedDay = null;
            this.sessions = [];
            this.errorMessage = seriesError;
            this.status = 'error';
            this.#inflight = null;
            return Promise.resolve();
        }
        this.selectedDay = mostRecentDayWithValue(series);
        return this.#load();
    }

    /** Switch to another day and load its sessions. */
    select(day: string): Promise<void> {
        if (day === this.selectedDay && this.status !== 'error' && this.status !== 'unavailable') {
            return Promise.resolve();
        }
        this.selectedDay = day;
        return this.#load();
    }

    /** Await the in-flight load (deterministic settling in tests). */
    settled(): Promise<void> {
        return this.#inflight ?? Promise.resolve();
    }

    async #load(): Promise<void> {
        const day = this.selectedDay;
        // No day with data → honest-empty, no request (nothing to scope to).
        if (!day) {
            this.sessions = [];
            this.errorMessage = '';
            this.status = 'empty';
            this.#inflight = null;
            return;
        }
        const token = ++this.#token;
        this.status = 'loading';
        this.errorMessage = '';
        const run = (async () => {
            const res = await this.#api.getProjectMetricDaySessions(
                this.#projectId,
                this.#metricKey,
                day,
            );
            // Drop a superseded / post-dispose response so we never render a
            // stale day's sessions under the current selection.
            if (this.#disposed || token !== this.#token) return;
            if (!res.ok) {
                this.sessions = [];
                if (res.error.status === 404) {
                    this.status = 'unavailable';
                } else {
                    this.errorMessage = res.error.message;
                    this.status = 'error';
                }
                return;
            }
            const sessions = res.data.sessions ?? [];
            this.sessions = sessions;
            this.status = sessions.length ? 'ready' : 'empty';
        })();
        this.#inflight = run;
        await run;
    }

    dispose(): void {
        this.#disposed = true;
    }
}
