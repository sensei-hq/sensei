import { describe, it, expect, beforeEach, vi } from 'vitest';
import { relayInboxState } from './relay-inbox-state.svelte';
import { relayInboxMock } from './relay-inbox.mock';
import { rankSession, filterSessions, needsCount } from './relay-inbox-view';
import type { RelaySession } from './types';

// Layer 2 test: drive the state's methods, assert its values — no DOM, no network.
// Plus the pure view logic (rank/filter) directly.

const base: RelaySession = {
	id: 'r',
	project: null,
	title: 't',
	goal: null,
	status: 'running',
	done: 0,
	total: 0,
	phase: null,
	lastEventAt: null,
	needs: 0,
	attention: null,
	plan: { phases: [] },
	asks: [],
	tenantKey: 'organization/acme',
	activity: []
};
const s = (over: Partial<RelaySession>): RelaySession => ({ ...base, ...over });

describe('relay-inbox-view (pure)', () => {
	it('ranks needs → attention → running → pending → done', () => {
		expect(rankSession(s({ needs: 1 }))).toBe(0);
		expect(rankSession(s({ attention: 'stalled', status: 'stalled' }))).toBe(1);
		expect(rankSession(s({ status: 'running' }))).toBe(2);
		expect(rankSession(s({ status: 'paused' }))).toBe(3);
		expect(rankSession(s({ status: 'done' }))).toBe(4);
	});

	it('filters by tab; needs = open asks OR attention', () => {
		const list = [
			s({ id: 'a', needs: 2, status: 'paused' }), // needs-you, not running (isolates the tabs)
			s({ id: 'b', status: 'stalled', attention: 'stalled' }),
			s({ id: 'c', status: 'running' }),
			s({ id: 'd', status: 'done' })
		];
		expect(filterSessions(list, 'needs').map((x) => x.id)).toEqual(['a', 'b']);
		expect(filterSessions(list, 'running').map((x) => x.id)).toEqual(['c']);
		expect(filterSessions(list, 'finished').map((x) => x.id)).toEqual(['d']);
		expect(filterSessions(list, 'all')).toHaveLength(4);
	});

	it('needsCount sums open asks', () => {
		expect(needsCount([s({ needs: 1 }), s({ needs: 2 }), s({ needs: 0 })])).toBe(3);
	});
});

describe('relayInboxState', () => {
	beforeEach(() => {
		relayInboxState.load([]);
		relayInboxState.select(null);
		relayInboxState.setFilter('needs');
	});

	it('load → sessions + needsCount; default filter shows only needs/attention', () => {
		relayInboxState.load(relayInboxMock());
		expect(relayInboxState.sessions).toHaveLength(7);
		expect(relayInboxState.needsCount).toBe(8); // 3+1+1+2+1 across the five gated runs
		// 'needs' filter: five gated + the stalled run = 6; the finished run is hidden
		expect(relayInboxState.shown).toHaveLength(6);
		expect(relayInboxState.shown[0].id).toBe('run-lumen');
		expect(relayInboxState.shown.some((x) => x.id === 'run-done')).toBe(false);
	});

	it('setFilter + select drive shown/selected', () => {
		relayInboxState.load(relayInboxMock());
		relayInboxState.setFilter('finished');
		expect(relayInboxState.shown.map((x) => x.id)).toEqual(['run-done']);
		relayInboxState.select('run-lumen');
		expect(relayInboxState.selected?.id).toBe('run-lumen');
	});

	it('patch upserts (realtime) and remove clears selection', () => {
		relayInboxState.load(relayInboxMock());
		relayInboxState.patch(s({ id: 'run-lumen', title: 'renamed', needs: 0, status: 'done' }));
		expect(relayInboxState.sessions.find((x) => x.id === 'run-lumen')?.title).toBe('renamed');
		expect(relayInboxState.needsCount).toBe(5); // lumen's 3 asks answered → 8 - 3
		relayInboxState.patch(s({ id: 'run-new', needs: 1 }));
		expect(relayInboxState.sessions).toHaveLength(8);
		relayInboxState.select('run-new');
		relayInboxState.remove('run-new');
		expect(relayInboxState.sessions).toHaveLength(7);
		expect(relayInboxState.selectedId).toBe(null);
	});
});

// #104: a relay failure rendered as an empty inbox, and nothing showed while loading.
describe('relayInboxState · load outcome', () => {
	const result = (over: Partial<Parameters<typeof relayInboxState.loaded>[0]> = {}) => ({
		sessions: [s({ id: 'a' })],
		tenants: 2,
		unreachable: [] as string[],
		plansMissing: 0,
		...over
	});

	it('is loading between begin() and the result', () => {
		relayInboxState.begin();
		expect(relayInboxState.status).toBe('loading');
	});

	it('a partial result is ready, keeps its runs, and names what is missing', () => {
		relayInboxState.begin();
		relayInboxState.loaded(result({ unreachable: ['two'], plansMissing: 1 }));
		expect(relayInboxState.status).toBe('ready');
		expect(relayInboxState.sessions.map((x) => x.id)).toEqual(['a']);
		expect(relayInboxState.unreachable).toEqual(['two']);
		expect(relayInboxState.plansMissing).toBe(1);
	});

	it('every dōjō unreachable is an error, not an empty inbox', () => {
		relayInboxState.begin();
		relayInboxState.loaded(result({ sessions: [], unreachable: ['one', 'two'] }));
		expect(relayInboxState.status).toBe('error');
		expect(relayInboxState.error).toBe('None of your 2 dōjōs could be reached.');
	});

	it('one dōjō unreachable reads in the singular', () => {
		relayInboxState.begin();
		relayInboxState.loaded(result({ sessions: [], tenants: 1, unreachable: ['one'] }));
		expect(relayInboxState.error).toBe('Your dōjō could not be reached.');
	});

	it('a thrown load is an error with its message', () => {
		relayInboxState.begin();
		relayInboxState.fail('relay timed out');
		expect(relayInboxState.status).toBe('error');
		expect(relayInboxState.error).toBe('relay timed out');
	});

	it('a membership-less inbox is ready and empty', () => {
		relayInboxState.begin();
		relayInboxState.loaded(result({ sessions: [], tenants: 0 }));
		expect(relayInboxState.status).toBe('ready');
		expect(relayInboxState.sessions).toEqual([]);
	});
});

// #305: an answer reaches the daemon, or the ask says why it did not.
describe('relayInboxState · answering an ask', () => {
	const approval = { id: 'g1', action: 'approve' as const, blocking: true, prompt: 'Run it?', options: ['Approve', 'Deny'], createdAt: '' };
	const run = s({ id: 'r1', asks: [approval], needs: 1 });
	beforeEach(() => relayInboxState.load([run]));

	it('sends the reply to the run’s dōjō and records the answer once it lands', async () => {
		const send = vi.fn().mockResolvedValue(undefined);
		await relayInboxState.answerAsk(send, run, 'g1', 'Deny', 'not in prod');
		expect(send).toHaveBeenCalledWith('organization/acme', 'g1', { verdict: 'deny', note: 'not in prod' });
		expect(relayInboxState.answered.g1).toBe('Deny');
		expect(relayInboxState.askError('g1')).toBeNull();
	});

	it('a reply that did not land records nothing and says why', async () => {
		await relayInboxState.answerAsk(vi.fn().mockRejectedValue(new Error('relay unavailable')), run, 'g1', 'Approve');
		expect(relayInboxState.answered.g1).toBeUndefined();
		expect(relayInboxState.askError('g1')).toBe('relay unavailable');
		expect(relayInboxState.isSending('g1')).toBe(false);
	});

	it('a run opened outside the inbox (a /you/runs link) is answered the same way', async () => {
		const elsewhere = s({ id: 'r9', asks: [{ ...approval, id: 'g9' }], needs: 1, tenantKey: 'organization/other' });
		const send = vi.fn().mockResolvedValue(undefined);
		await relayInboxState.answerAsk(send, elsewhere, 'g9', 'Approve');
		expect(send).toHaveBeenCalledWith('organization/other', 'g9', { verdict: 'approve' });
	});

	it('an ask the run does not hold is not sent', async () => {
		const send = vi.fn();
		await relayInboxState.answerAsk(send, run, 'nope', 'Approve');
		expect(send).not.toHaveBeenCalled();
		expect(relayInboxState.askError('nope')).toBeTruthy();
	});
});
