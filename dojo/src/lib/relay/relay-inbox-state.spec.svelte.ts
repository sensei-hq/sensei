import { describe, it, expect, beforeEach } from 'vitest';
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
	asks: []
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
		expect(relayInboxState.sessions).toHaveLength(4);
		expect(relayInboxState.needsCount).toBe(2); // run-approve + run-choose
		// default 'needs' filter: the two gated runs + the stalled run, not the done run
		expect(relayInboxState.shown.map((x) => x.id)).toEqual(['run-approve', 'run-choose', 'run-stalled']);
	});

	it('setFilter + select drive shown/selected', () => {
		relayInboxState.load(relayInboxMock());
		relayInboxState.setFilter('finished');
		expect(relayInboxState.shown.map((x) => x.id)).toEqual(['run-done']);
		relayInboxState.select('run-approve');
		expect(relayInboxState.selected?.id).toBe('run-approve');
	});

	it('patch upserts (realtime) and remove clears selection', () => {
		relayInboxState.load(relayInboxMock());
		relayInboxState.patch(s({ id: 'run-approve', title: 'renamed', needs: 0, status: 'done' }));
		expect(relayInboxState.sessions.find((x) => x.id === 'run-approve')?.title).toBe('renamed');
		expect(relayInboxState.needsCount).toBe(1); // one gate answered
		relayInboxState.patch(s({ id: 'run-new', needs: 1 }));
		expect(relayInboxState.sessions).toHaveLength(5);
		relayInboxState.select('run-new');
		relayInboxState.remove('run-new');
		expect(relayInboxState.sessions).toHaveLength(4);
		expect(relayInboxState.selectedId).toBe(null);
	});
});
