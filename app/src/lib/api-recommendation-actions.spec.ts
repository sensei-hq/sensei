import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi } from './api.js';

// L3 · ActionItems act (review r1 M5): the action items' writes read the
// daemon's `{ error }` on a refusal, so a person sees WHY ("already decided",
// "a skill named … already exists") instead of the status text "Conflict".

const response = (status: number, statusText: string, body: unknown) =>
    ({ ok: status < 400, status, statusText, json: async () => body }) as unknown as Response;

afterEach(() => vi.unstubAllGlobals());

const api = () => senseiApi(7744);
const writes = [
    {
        name: 'trySnoozeWorkspaceRecommendation',
        call: () => api().trySnoozeWorkspaceRecommendation('w1', 'r1', 7),
        path: '/api/workspaces/w1/recommendations/r1/snooze',
        body: { days: 7 },
    },
    {
        name: 'tryMaterializeRecommendationWithReason',
        call: () => api().tryMaterializeRecommendationWithReason('w1', 'r1', { gov_scope: 'workspace', enforcement: 'advisory' }),
        path: '/api/workspaces/w1/recommendations/r1/materialize',
        body: { gov_scope: 'workspace', enforcement: 'advisory' },
    },
];

describe.each(writes)('$name', ({ call, path, body }) => {
    it('posts its body as JSON to its route and returns the data', async () => {
        const fetch = vi.fn(async () => response(200, 'OK', { ok: true }));
        vi.stubGlobal('fetch', fetch);
        expect(await call()).toEqual({ ok: true, data: { ok: true } });
        const [url, init] = fetch.mock.calls[0] as unknown as [string, RequestInit];
        expect(url).toBe(`http://127.0.0.1:7744${path}`);
        expect(init.method).toBe('POST');
        expect(JSON.parse(init.body as string)).toEqual(body);
    });

    it('a refusal carries the daemon’s reason, not the status text', async () => {
        vi.stubGlobal('fetch', vi.fn(async () =>
            response(409, 'Conflict', { error: "a skill named 'retry-budget' already exists in this repo; edit the title" })));
        const r = await call();
        expect(r.ok).toBe(false);
        if (!r.ok) {
            expect(r.error.status).toBe(409);
            expect(r.error.message).toBe("a skill named 'retry-budget' already exists in this repo; edit the title");
        }
    });

    it('an unreachable daemon is an error, never a success', async () => {
        vi.stubGlobal('fetch', vi.fn(async () => { throw new Error('connection refused'); }));
        expect(await call()).toEqual({ ok: false, error: { status: 0, message: 'connection refused' } });
    });
});
