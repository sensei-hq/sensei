import { describe, it, expect, vi } from 'vitest';
import { listRuns, getSegments, submitReview, replyToGate, listGates, sendNudge, DojoApiError } from './relay-data';

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

describe('relay-data', () => {
	it('listRuns hits relay/session with the bearer token and returns sessions', async () => {
		const calls: { url: string; init?: RequestInit }[] = [];
		const fetch = vi.fn(async (url: string, init?: RequestInit) => {
			calls.push({ url, init });
			return jsonResponse({
				sessions: [
					{
						id: 'r1',
						run_id: 'run-1',
						title: 'x',
						goal: null,
						status: 'running',
						progress_done: 1,
						progress_total: 3,
						current_phase: null,
						current_feature: null,
						last_event_at: null,
						paused_until: null,
						pause_reason: null,
						heartbeat_at: 't-hb',
						started_at: 't',
						completed_at: null
					}
				]
			});
		}) as unknown as typeof globalThis.fetch;
		const runs = await listRuns('personal/jerry', { accessToken: 'JWT', fetch });
		expect(runs).toHaveLength(1);
		expect(runs[0].run_id).toBe('run-1');
		expect(calls[0].url).toContain('/v1/t/personal/jerry/relay/session');
		expect((calls[0].init?.headers as Record<string, string>).Authorization).toBe('Bearer JWT');
	});

	it('getSegments encodes run_id and returns the ordered outline', async () => {
		let seenUrl = '';
		const fetch = vi.fn(async (url: string) => {
			seenUrl = url;
			return jsonResponse({
				segments: [{ id: 's0', parent_id: null, seq: 0, title: 'Phase 1', summary: null, detail: null, state: 'active', is_gate: false, gate_severity: null, response_verdict: null, response_note: null, submitted_at: null }]
			});
		}) as unknown as typeof globalThis.fetch;
		const segs = await getSegments('personal/jerry', 'run 1', { fetch });
		expect(segs[0].title).toBe('Phase 1');
		expect(seenUrl).toContain('run_id=run%201');
	});

	it('submitReview posts the batch and returns the count applied', async () => {
		let sent: { run_id?: string; reviews?: unknown[] } = {};
		const fetch = vi.fn(async (_url: string, init?: RequestInit) => {
			sent = JSON.parse(String(init?.body));
			return jsonResponse({ submitted: 2 });
		}) as unknown as typeof globalThis.fetch;
		const n = await submitReview(
			'personal/jerry',
			'run-1',
			[
				{ seq: 0, verdict: 'approve' },
				{ seq: 1, verdict: 'request_changes', note: 'add a test' }
			],
			{ accessToken: 'JWT', fetch }
		);
		expect(n).toBe(2);
		expect(sent.run_id).toBe('run-1');
		expect(sent.reviews).toHaveLength(2);
	});

	it('listGates hits relay/gates with the bearer token and returns pending gates', async () => {
		const calls: { url: string; init?: RequestInit }[] = [];
		const fetch = vi.fn(async (url: string, init?: RequestInit) => {
			calls.push({ url, init });
			return jsonResponse({
				gates: [
					{
						id: 'g1',
						seq: 7,
						run_id: 'run-1',
						run_title: 'Round-trip',
						segment_id: null,
						kind: 'approval',
						payload: { prompt: 'run cargo test?' },
						created_at: 't'
					}
				]
			});
		}) as unknown as typeof globalThis.fetch;
		const gates = await listGates('personal/jerry', { accessToken: 'JWT', fetch });
		expect(gates).toHaveLength(1);
		expect(gates[0].id).toBe('g1');
		expect(gates[0].run_id).toBe('run-1');
		expect(calls[0].url).toContain('/v1/t/personal/jerry/relay/gates');
		expect((calls[0].init?.headers as Record<string, string>).Authorization).toBe('Bearer JWT');
	});

	it('listGates returns [] when the body has no gates', async () => {
		const fetch = vi.fn(async () => jsonResponse({})) as unknown as typeof globalThis.fetch;
		expect(await listGates('personal/jerry', { fetch })).toEqual([]);
	});

	it('replyToGate posts inbox_id + reply', async () => {
		let sent: { inbox_id?: string } = {};
		const fetch = vi.fn(async (_url: string, init?: RequestInit) => {
			sent = JSON.parse(String(init?.body));
			return jsonResponse({ id: 'i1', seq: 5 });
		}) as unknown as typeof globalThis.fetch;
		await replyToGate('personal/jerry', 'i1', { verdict: 'approve' }, { fetch });
		expect(sent.inbox_id).toBe('i1');
	});

	it('sendNudge posts run_id + text + kind (defaults to nudge) and returns id/seq', async () => {
		let sent: { run_id?: string; text?: string; kind?: string } = {};
		const fetch = vi.fn(async (_url: string, init?: RequestInit) => {
			sent = JSON.parse(String(init?.body));
			return jsonResponse({ id: 'n1', seq: 12 });
		}) as unknown as typeof globalThis.fetch;
		const out = await sendNudge('personal/jerry', 'run-1', 'focus on the API first', {
			accessToken: 'JWT',
			fetch
		});
		expect(out).toEqual({ id: 'n1', seq: 12 });
		expect(sent.run_id).toBe('run-1');
		expect(sent.text).toBe('focus on the API first');
		expect(sent.kind).toBe('nudge'); // server-side trim of whitespace is covered by the harness
	});

	it('throws DojoApiError on a non-2xx', async () => {
		const fetch = vi.fn(async () => jsonResponse({ error: 'forbidden' }, 403)) as unknown as typeof globalThis.fetch;
		await expect(listRuns('personal/jerry', { fetch })).rejects.toBeInstanceOf(DojoApiError);
	});
});
