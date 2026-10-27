// /v1/you/preferences — the signed-in person's display preferences (#133),
// read and written as them; the app's daemon syncs through it.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), readPreferences: vi.fn(), writeGlyphs: vi.fn() }));
const SESSION = vi.hoisted(() => ({ as: 'session' }));

vi.mock('$lib/server/caller-db', () => ({ callerDb: () => SESSION }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/preferences', () => ({ readPreferences: mocks.readPreferences, writeGlyphs: mocks.writeGlyphs }));

const { GET, PUT } = await import('./+server');
const ev = (body?: unknown) =>
	({
		request: new Request('http://x/v1/you/preferences', body ? { method: 'PUT', body: JSON.stringify(body) } : {}),
		locals: {}
	}) as never;

beforeEach(() => {
	mocks.resolveCaller.mockReset().mockResolvedValue({ userId: 'p1' });
	mocks.readPreferences.mockReset().mockResolvedValue({ glyphs: 'icon', glyphs_set_at: '2026-10-27T10:00:00Z' });
	mocks.writeGlyphs.mockReset().mockImplementation(async (_db, _u, p) => p);
});

describe('/v1/you/preferences', () => {
	it('reads the caller’s preferences, as them', async () => {
		const res = await GET(ev());
		expect(await res.json()).toEqual({ glyphs: 'icon', glyphs_set_at: '2026-10-27T10:00:00Z' });
		expect(mocks.readPreferences).toHaveBeenCalledWith(SESSION, 'p1');
	});

	it('writes a choice and answers what is stored', async () => {
		const body = { glyphs: 'kanji', glyphs_set_at: '2026-10-27T11:00:00Z' };
		const res = await PUT(ev(body));
		expect([res.status, await res.json()]).toEqual([200, body]);
		expect(mocks.writeGlyphs).toHaveBeenCalledWith(SESSION, 'p1', body);
	});

	it('refuses a mode other than kanji or icon, and a choice without its time', async () => {
		expect((await PUT(ev({ glyphs: 'emoji', glyphs_set_at: '2026-10-27T11:00:00Z' }))).status).toBe(400);
		expect((await PUT(ev({ glyphs: 'icon' }))).status).toBe(400);
		expect((await PUT(ev({ glyphs: 'icon', glyphs_set_at: 'yesterday' }))).status).toBe(400);
		expect(mocks.writeGlyphs).not.toHaveBeenCalled();
	});

	it('propagates the auth refusal', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('{}', { status: 401 }));
		expect((await GET(ev())).status).toBe(401);
	});
});
