import { describe, it, expect, vi } from 'vitest';
import { loadRelayInbox } from './relay-inbox';

// Layer 3: the fan-out over memberships (#104). A dōjō whose relay fails used to
// return [] — indistinguishable from a dōjō with no runs — so an outage rendered as
// an empty inbox. The load keeps every run that did load and names what did not.

function json(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

const run = (run_id: string) => ({
	id: `db-${run_id}`,
	run_id,
	title: run_id,
	goal: null,
	status: 'running',
	progress_done: 0,
	progress_total: 1,
	current_phase: null,
	current_feature: null,
	last_event_at: null,
	paused_until: null,
	pause_reason: null,
	heartbeat_at: null,
	started_at: 't',
	completed_at: null
});

/** A relay where `down` tenants answer 503 and `noPlan` runs fail their segments read. */
function relay({ down = [] as string[], noPlan = [] as string[] } = {}) {
	return vi.fn(async (url: string) => {
		const tenant = down.find((t) => url.includes(`/v1/t/${t}/`));
		if (tenant) return json({ error: 'unavailable' }, 503);
		if (url.includes('/relay/session')) {
			const t = url.match(/\/v1\/t\/([^/]+)\//)?.[1] ?? '';
			return json({ sessions: [run(`${t}-run`)] });
		}
		if (url.includes('/relay/gates')) return json({ gates: [] });
		if (url.includes('/relay/segments')) {
			const id = decodeURIComponent(url.match(/run_id=([^&]+)/)?.[1] ?? '');
			return noPlan.includes(id) ? json({ error: 'boom' }, 500) : json({ segments: [] });
		}
		return json({}, 404);
	}) as unknown as typeof fetch;
}

const memberships = [{ url: 'one' }, { url: 'two' }];

describe('loadRelayInbox', () => {
	it('keeps the runs of the dōjōs that answered and names the one that did not', async () => {
		const got = await loadRelayInbox({ memberships, accessToken: 'JWT', fetch: relay({ down: ['two'] }) });
		expect(got.sessions.map((s) => s.id)).toEqual(['one-run']);
		expect(got.unreachable).toEqual(['two']);
	});

	it('counts a run whose plan did not load, and keeps the run', async () => {
		const got = await loadRelayInbox({ memberships, accessToken: 'JWT', fetch: relay({ noPlan: ['two-run'] }) });
		expect(got.sessions.map((s) => s.id).sort()).toEqual(['one-run', 'two-run']);
		expect(got.plansMissing).toBe(1);
		expect(got.unreachable).toEqual([]);
	});

	it('reports nothing missing when every dōjō answers', async () => {
		const got = await loadRelayInbox({ memberships, accessToken: 'JWT', fetch: relay() });
		expect(got).toMatchObject({ unreachable: [], plansMissing: 0 });
		expect(got.sessions).toHaveLength(2);
	});
});
