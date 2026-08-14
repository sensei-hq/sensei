// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { DaySessions, DrilldownSession, MetricSeriesPoint } from '$lib/metrics/metric-view.js';
import { DatapointDrilldownState } from './datapoint-drilldown.svelte.js';
import DatapointDrilldownHarness from './DatapointDrilldown.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
    cleanup.forEach((fn) => fn());
    cleanup = [];
});

const q = (root: HTMLElement, sel: string) => root.querySelector(sel) as HTMLElement | null;
const qa = (root: HTMLElement, sel: string) =>
    Array.from(root.querySelectorAll(sel)) as HTMLElement[];

function pt(period: string, value: number | null, explainer?: string | null): MetricSeriesPoint {
    return { period, value, direction: 'higher_better', explainer };
}

function session(over: Partial<DrilldownSession> = {}): DrilldownSession {
    return {
        client_session_id: 'abcdef1234567890',
        started_at: '2026-08-11T09:00:00Z',
        outcome: 'completed',
        ftr: true,
        turns: 3,
        corrections: 0,
        task: 'Add the day-scoped drill-down',
        summary: 'Wired the datapoint drill-down end to end.',
        observation: { title: 'First-try rate', detail: 'outcome completed; first-try; 3 turns' },
        evidence: null,
        resumed: false,
        ...over,
    };
}

// A series with two days that have data (Aug 9, Aug 11) and one gap day (Aug 10,
// null) that must NOT appear in the selector. Aug 11 carries the explainer.
const SERIES: MetricSeriesPoint[] = [
    pt('2026-08-09', 0.5, 'FTR held at 50% — one of two sessions needed rework.'),
    pt('2026-08-10', null),
    pt('2026-08-11', 1, 'FTR was 100% — both sessions landed first-try.'),
];

/** A controller whose api resolves `getProjectMetricDaySessions` per day via the
 *  supplied resolver — so each test drives the ready / empty / unavailable /
 *  error branch deterministically. */
function controllerWith(
    resolve: (day: string) => ApiResult<DaySessions>,
): { controller: DatapointDrilldownState; fetch: ReturnType<typeof vi.fn> } {
    const fetch = vi.fn((_id: string, _key: string, day: string) =>
        Promise.resolve(resolve(day)),
    );
    const api = { getProjectMetricDaySessions: fetch } as unknown as SenseiApi;
    return { controller: new DatapointDrilldownState(api), fetch };
}

const ok = (day: string, sessions: DrilldownSession[]): ApiResult<DaySessions> => ({
    ok: true,
    data: { metric: 'ftr', day, sessions, count: sessions.length },
});

// Mount the harness and settle onMount's init() + the load it kicks off. The
// manual init is deterministic (same args the harness passed); the extra
// tick/settle covers onMount firing on either ordering.
async function mountLoaded(
    controller: DatapointDrilldownState,
    series: MetricSeriesPoint[] = SERIES,
    projectId = 'proj-1',
    metricKey = 'ftr',
) {
    const m = mountComponent(DatapointDrilldownHarness, { controller, series, projectId, metricKey });
    cleanup.push(m.destroy);
    await controller.init(series, projectId, metricKey);
    await tick();
    await controller.settled();
    await tick();
    return m;
}

describe('DatapointDrilldown', () => {
    it('renders the eyebrow header', async () => {
        const { controller } = controllerWith((day) => ok(day, [session()]));
        const m = await mountLoaded(controller);
        expect(m.container.querySelector('[data-component="datapoint-drilldown"]')).toBeTruthy();
        expect(m.container.textContent).toContain('Behind this datapoint');
    });

    it('defaults to the most recent day with data and offers only days with data', async () => {
        const { controller } = controllerWith((day) => ok(day, [session()]));
        const m = await mountLoaded(controller);
        const selector = q(m.container, '[data-drilldown-day-selector]');
        expect(selector).toBeTruthy();
        const days = qa(m.container, '[data-day]').map((b) => b.getAttribute('data-day'));
        // Aug 10 is a null-value gap — it must not be selectable.
        expect(days).toEqual(['2026-08-09', '2026-08-11']);
        // The most recent day is the default selection.
        const active = qa(m.container, '[data-day]').filter(
            (b) => b.getAttribute('data-active') === 'true',
        );
        expect(active).toHaveLength(1);
        expect(active[0].getAttribute('data-day')).toBe('2026-08-11');
        // The label is the compact, locale-free form.
        expect(active[0].textContent?.trim()).toBe('Aug 11');
    });

    it("shows the selected day's explainer from the series (never fabricated)", async () => {
        const { controller } = controllerWith((day) => ok(day, [session()]));
        const m = await mountLoaded(controller);
        expect(q(m.container, '[data-drilldown-explainer]')?.textContent).toContain(
            'both sessions landed first-try',
        );
    });

    it('renders each session — short id, structural one-liner, task/summary, observation', async () => {
        const { controller, fetch } = controllerWith((day) => ok(day, [session()]));
        const m = await mountLoaded(controller);
        expect(fetch).toHaveBeenCalledWith('proj-1', 'ftr', '2026-08-11');
        const row = q(m.container, '[data-session="abcdef1234567890"]');
        expect(row).toBeTruthy();
        expect(q(row!, '.font-mono')?.textContent?.trim()).toBe('abcdef12');
        expect(q(m.container, '[data-session-meta]')?.textContent?.trim()).toBe(
            'completed · first-try · 3 turns',
        );
        expect(row!.textContent).toContain('Add the day-scoped drill-down');
        expect(row!.textContent).toContain('Wired the datapoint drill-down end to end.');
        const obs = q(m.container, '[data-session-observation]');
        expect(obs?.textContent).toContain('First-try rate');
        expect(obs?.textContent).toContain('outcome completed; first-try; 3 turns');
    });

    it('reads a corrected session as its correction count, not first-try', async () => {
        const corrected = session({
            client_session_id: 'beef00001111',
            outcome: 'corrected',
            ftr: false,
            turns: 4,
            corrections: 2,
        });
        const { controller } = controllerWith((day) => ok(day, [corrected]));
        const m = await mountLoaded(controller);
        expect(q(m.container, '[data-session-meta]')?.textContent?.trim()).toBe(
            'corrected · 2 corrections · 4 turns',
        );
    });

    it('honest-empty: a day with no measurable session shows the plain empty line', async () => {
        const { controller } = controllerWith((day) => ok(day, []));
        const m = await mountLoaded(controller);
        expect(q(m.container, '[data-component="datapoint-drilldown"]')?.getAttribute('data-status')).toBe(
            'empty',
        );
        expect(q(m.container, '[data-drilldown-empty]')?.textContent).toContain(
            'No measurable sessions that day.',
        );
        expect(q(m.container, '[data-session]')).toBeNull();
    });

    it('a 404 (endpoint absent) shows the not-available notice — never a fabricated session', async () => {
        const { controller } = controllerWith(() => ({
            ok: false,
            error: { status: 404, message: 'Not Found' },
        }));
        const m = await mountLoaded(controller);
        expect(q(m.container, '[data-component="datapoint-drilldown"]')?.getAttribute('data-status')).toBe(
            'unavailable',
        );
        expect(q(m.container, '[data-drilldown-unavailable]')?.textContent).toContain(
            "isn’t available on the connected daemon yet",
        );
        expect(q(m.container, '[data-session]')).toBeNull();
    });

    it('a real error surfaces the daemon message — never a fabricated session', async () => {
        const { controller } = controllerWith(() => ({
            ok: false,
            error: { status: 500, message: 'Internal Server Error' },
        }));
        const m = await mountLoaded(controller);
        expect(q(m.container, '[data-component="datapoint-drilldown"]')?.getAttribute('data-status')).toBe(
            'error',
        );
        expect(q(m.container, '[data-drilldown-error]')?.textContent).toContain(
            'Internal Server Error',
        );
        expect(q(m.container, '[data-session]')).toBeNull();
    });

    it('selecting another day refetches that day and moves the active marker', async () => {
        const bySession: Record<string, DrilldownSession> = {
            '2026-08-11': session(),
            '2026-08-09': session({
                client_session_id: 'beef00001111',
                task: 'The earlier session',
                outcome: 'corrected',
                ftr: false,
                corrections: 1,
                turns: 2,
            }),
        };
        const { controller, fetch } = controllerWith((day) => ok(day, [bySession[day]]));
        const m = await mountLoaded(controller);
        expect(fetch).toHaveBeenLastCalledWith('proj-1', 'ftr', '2026-08-11');

        const earlier = q(m.container, '[data-day="2026-08-09"]')!;
        earlier.click();
        await controller.settled();
        await tick();

        expect(fetch).toHaveBeenLastCalledWith('proj-1', 'ftr', '2026-08-09');
        const active = qa(m.container, '[data-day]').filter(
            (b) => b.getAttribute('data-active') === 'true',
        );
        expect(active).toHaveLength(1);
        expect(active[0].getAttribute('data-day')).toBe('2026-08-09');
        expect(q(m.container, '[data-session="beef00001111"]')?.textContent).toContain(
            'The earlier session',
        );
    });

    it('a series with no day-with-data shows the empty line and never calls the endpoint', async () => {
        const { controller, fetch } = controllerWith((day) => ok(day, [session()]));
        const m = await mountLoaded(controller, [pt('2026-08-10', null)]);
        expect(q(m.container, '[data-drilldown-day-selector]')).toBeNull();
        expect(q(m.container, '[data-drilldown-empty]')).toBeTruthy();
        expect(fetch).not.toHaveBeenCalled();
    });

    it('surfaces an upstream daily-series FETCH failure as an error — never masked honest-empty', async () => {
        // The loader passes seriesError when the daily-series fetch failed. The
        // drill-down must show the error state, NOT "No measurable sessions" (which
        // would be indistinguishable from a genuinely-empty day). No per-day fetch
        // is made — there is no series to scope to.
        const { controller, fetch } = controllerWith((day) => ok(day, [session()]));
        await controller.init(SERIES, 'proj-1', 'ftr', 'Internal Server Error');
        expect(controller.status).toBe('error');
        expect(controller.errorMessage).toContain('Internal Server Error');
        expect(controller.selectedDay).toBeNull();
        expect(fetch).not.toHaveBeenCalled();
    });

    it('shows the loading state while a day’s sessions are in flight', async () => {
        let release!: (r: ApiResult<DaySessions>) => void;
        const fetch = vi.fn(
            () => new Promise<ApiResult<DaySessions>>((r) => { release = r; }),
        );
        const api = { getProjectMetricDaySessions: fetch } as unknown as SenseiApi;
        const controller = new DatapointDrilldownState(api);
        const m = mountComponent(DatapointDrilldownHarness, {
            controller, series: SERIES, projectId: 'proj-1', metricKey: 'ftr',
        });
        cleanup.push(m.destroy);
        await tick(); // onMount → init() kicks the per-day fetch, which never resolves yet
        expect(
            q(m.container, '[data-component="datapoint-drilldown"]')?.getAttribute('data-status'),
        ).toBe('loading');
        expect(q(m.container, '[data-drilldown-loading]')?.textContent).toContain('Reading that day');
        release(ok('2026-08-11', [session()])); // settle so afterEach teardown is clean
        await controller.settled();
    });
});
