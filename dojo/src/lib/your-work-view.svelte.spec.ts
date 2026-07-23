import { describe, expect, it } from 'vitest';
import {
	orderNeeds,
	primaryLabel,
	actForKind,
	createYourWork,
	K2_URGENCY
} from './your-work-view.svelte';
import type { KitNeed } from './components/kit/types';

// The Your-work landing state (chunk 1, ScrYourWork). The needs-you band is a
// remote control: pure `orderNeeds` sorts by urgency, `primaryLabel`/`actForKind`
// name the lead action per kind, and `createYourWork` owns the `resolved` rune
// map + the derived open-count / next-need / stats. Ported from the mockup
// (dojo2-app.jsx ScrYourWork · K2_URGENCY · K2_PRIMARY).

const needs: KitNeed[] = [
	{ id: 'r', kind: 'review', title: 'r', project: 'p', dojo: 'd', why: 'w', age: '2h' },
	{ id: 'd', kind: 'decision', title: 'd', project: 'p', dojo: 'd', why: 'w', age: '1h' },
	{ id: 'g', kind: 'gate', title: 'g', project: 'p', dojo: 'd', why: 'w', age: '3m' },
	{ id: 'c', kind: 'conflict', title: 'c', project: 'p', dojo: 'd', why: 'w', age: '26m' }
];

describe('orderNeeds — urgency ordering (gate → conflict → decision → review)', () => {
	it('sorts by K2_URGENCY rank', () => {
		expect(orderNeeds(needs).map((n) => n.kind)).toEqual([
			'gate',
			'conflict',
			'decision',
			'review'
		]);
	});

	it('is stable for equal ranks and does not mutate the input', () => {
		const input = needs.slice();
		orderNeeds(input);
		expect(input.map((n) => n.id)).toEqual(['r', 'd', 'g', 'c']);
	});

	it('ranks gate most-urgent, review least', () => {
		expect(K2_URGENCY.gate).toBeLessThan(K2_URGENCY.conflict);
		expect(K2_URGENCY.conflict).toBeLessThan(K2_URGENCY.decision);
		expect(K2_URGENCY.decision).toBeLessThan(K2_URGENCY.review);
	});
});

describe('primaryLabel / actForKind — the lead action per kind', () => {
	it('names the primary verb per kind (mockup K2_PRIMARY)', () => {
		expect(primaryLabel('gate')).toBe('Approve');
		expect(primaryLabel('conflict')).toBe('Settle');
		expect(primaryLabel('decision')).toBe('Decide');
		expect(primaryLabel('review')).toBe('Approve');
	});

	it('maps a kind to the act id fired through onAct', () => {
		expect(actForKind('conflict')).toBe('settle');
		expect(actForKind('decision')).toBe('decide');
		expect(actForKind('gate')).toBe('approve');
		expect(actForKind('review')).toBe('approve');
	});
});

describe('createYourWork — the rune-backed landing state', () => {
	it('starts with all needs open and the most-urgent as `next`', () => {
		const s = createYourWork(needs);
		expect(s.openItems.length).toBe(4);
		expect(s.next?.kind).toBe('gate');
	});

	it('resolving a need drops it from the open count and advances `next`', () => {
		const s = createYourWork(needs);
		s.act(s.next!, 'approve');
		expect(s.openItems.length).toBe(3);
		expect(s.next?.kind).toBe('conflict');
		expect(s.resolved['g']).toBe('approved');
	});

	it('maps deny/settle/decide act ids to their resolved labels', () => {
		const s = createYourWork(needs);
		s.act(needs[3], 'settle'); // conflict
		s.act(needs[1], 'decide'); // decision
		s.act(needs[0], 'deny'); // review
		expect(s.resolved).toMatchObject({ c: 'settled', d: 'decided', r: 'denied' });
	});

	it('reports zero open + no next when everything is resolved (calm state)', () => {
		const s = createYourWork(needs);
		for (const n of needs) s.act(n, actForKind(n.kind));
		expect(s.openItems.length).toBe(0);
		expect(s.next).toBeUndefined();
	});

	it('derives the week stats from the supplied projects + runs', () => {
		const s = createYourWork(needs, {
			projects: [{ runsWeek: 14 }, { runsWeek: 9 }, { runsWeek: 3 }],
			runs: [{ state: 'running' }, { state: 'waiting' }, { state: 'running' }]
		});
		expect(s.runsWeek).toBe(26);
		expect(s.activeRuns).toBe(2);
	});

	it('treats a missing runsWeek as zero', () => {
		const s = createYourWork([], { projects: [{ runsWeek: 5 }, {}], runs: [] });
		expect(s.runsWeek).toBe(5);
		expect(s.activeRuns).toBe(0);
	});
});
