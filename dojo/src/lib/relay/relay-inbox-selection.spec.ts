import { describe, expect, it } from 'vitest';
import { initialSelection } from './relay-inbox-view';

// #293: a gate card links to /you/runs/<id>, but the inbox opened its first run
// whatever the link named — the deep-linked run was never the one shown.
const s = (id: string) => ({ id }) as never;

describe('initialSelection', () => {
	it('opens the run the link names when the inbox has it, even if filtered out', () => {
		expect(initialSelection([s('a'), s('b'), s('c')], [s('a')], 'c')).toBe('c');
	});

	it('opens nothing when the linked run is not in the inbox, so the route shows it', () => {
		expect(initialSelection([s('a')], [s('a')], 'zz')).toBeNull();
	});

	it('opens the first shown run when no run is named', () => {
		expect(initialSelection([s('a'), s('b')], [s('b'), s('a')], undefined)).toBe('b');
		expect(initialSelection([], [], undefined)).toBeNull();
	});
});
