import { describe, expect, it } from 'vitest';
import { classTone, phaseTone, roleTone, kindTone, K2_NODE, taskState, nodeTone } from './index';

// The vocab maps decide every tone in one place; the lookups default safely.
describe('kit vocab lookups', () => {
	it('classTone resolves a known kind and defaults to company', () => {
		expect(classTone('client').label).toBe('client');
		expect(classTone('client').text).toBe('text-accent');
		expect(classTone(undefined).label).toBe('company');
		expect(classTone('nope').text).toBe('text-ink-mute');
	});

	it('phaseTone lights the right number of steps and defaults to watch', () => {
		expect(phaseTone('watch').step).toBe(1);
		expect(phaseTone('notice').step).toBe(2);
		expect(phaseTone('adopt').step).toBe(3);
		expect(phaseTone(null).label).toBe('watch');
	});

	it('roleTone returns a tone for a known role and undefined otherwise', () => {
		expect(roleTone('maintainer')?.label).toBe('maintainer');
		expect(roleTone('maintainer')?.icon).toBe('settings');
		expect(roleTone('ghost')).toBeUndefined();
		expect(roleTone(null)).toBeUndefined();
	});

	it('kindTone resolves org kinds and defaults to employer', () => {
		expect(kindTone('team').kanji).toBe('組');
		expect(kindTone('client').text).toBe('text-accent');
		expect(kindTone('community').text).toBe('text-success');
		expect(kindTone(undefined).kanji).toBe('社');
	});

	it('K2_NODE covers the seven states with token classes, never a raw color', () => {
		const states = ['done', 'active', 'needs_review', 'blocked', 'failed', 'skipped', 'pending'];
		expect(Object.keys(K2_NODE).sort()).toEqual([...states].sort());
		for (const tone of Object.values(K2_NODE)) {
			for (const cls of [tone.text, tone.fill, tone.soft, tone.edge]) {
				expect(cls).not.toMatch(/var\(|#|oklch|rgb/); // token classes only
			}
		}
		expect(K2_NODE.pending.dashed).toBe(true);
	});

	// A state's text is read on paper (labels, list icons) and on its own soft tint (the plan
	// node). The tint stays light in both modes, so the bright role is unreadable on it; the
	// node uses `onSoft`, the role darkened (rokkit text-on-{role}-soft).
	it('K2_NODE gives each state a paper tone and a tone for its own tint', () => {
		expect(K2_NODE.active).toMatchObject({ text: 'text-success', onSoft: 'text-on-success-soft' });
		expect(K2_NODE.needs_review).toMatchObject({ text: 'text-accent', onSoft: 'text-on-accent-soft' });
		expect(K2_NODE.blocked).toMatchObject({ text: 'text-warning', onSoft: 'text-on-warning-soft' });
		expect(K2_NODE.failed).toMatchObject({ text: 'text-danger', onSoft: 'text-on-danger-soft' });
		for (const state of ['done', 'skipped', 'pending'] as const)
			expect(K2_NODE[state].onSoft, state).toBe(K2_NODE[state].text);
	});

	it('taskState aliases legacy states and defaults unknowns to pending', () => {
		expect(taskState('running')).toBe('active'); // legacy alias
		expect(taskState('queued')).toBe('pending');
		expect(taskState('gate')).toBe('needs_review');
		expect(taskState('failed')).toBe('failed'); // canonical passes through
		expect(taskState('bogus')).toBe('pending'); // unknown → safe default
		expect(taskState(undefined)).toBe('pending');
	});

	it('nodeTone resolves via the alias and defaults to pending', () => {
		expect(nodeTone('running').label).toBe('active');
		expect(nodeTone('nope')).toBe(K2_NODE.pending);
	});
});
