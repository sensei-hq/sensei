import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrInbox from './ScrInbox.svelte';
import type { KitInbox } from '$lib/components/kit/types';

// ScrInbox is the left rail of the two-panel inbox: it renders the sorted list
// and marks the row whose run is open in the detail panel. The row copy/ranking
// is locked in InboxRow/relay-map specs; this locks the selection wiring.
function row(id: string): KitInbox {
	return {
		run: {
			id,
			project: 'lumen-auth',
			assistant: '',
			state: 'running',
			task: `task ${id}`,
			elapsed: '1m',
			edits: 0,
			last: '4m'
		},
		// needs > 0 so the row survives the default "needs you" filter.
		status: 'running',
		needs: 1,
		attention: 'gate',
		rank: 0,
		done: 1,
		total: 3
	};
}

afterEach(cleanup);

describe('ScrInbox', () => {
	it('marks the selected row and leaves the others unmarked', () => {
		const { getByText } = render(ScrInbox, {
			inbox: [row('r1'), row('r2')],
			selectedId: 'r2',
			onOpen: () => {}
		});
		const wrapper = (t: string) => getByText(t).closest('div.border-b');
		expect(wrapper('task r2')?.classList.contains('bg-paper-mute')).toBe(true);
		expect(wrapper('task r1')?.classList.contains('bg-paper-mute')).toBe(false);
	});

	it('marks nothing when no row is selected', () => {
		const { getByText } = render(ScrInbox, {
			inbox: [row('r1'), row('r2')],
			onOpen: () => {}
		});
		expect(getByText('task r1').closest('div.border-b')?.classList.contains('bg-paper-mute')).toBe(
			false
		);
	});
});
