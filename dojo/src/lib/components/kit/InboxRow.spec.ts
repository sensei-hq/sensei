import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import InboxRow from './InboxRow.svelte';
import type { KitInbox } from './types';

// InboxRow renders one in-flight session: the why-surfaced line (needs-you /
// attention / status label), the done/total, and opens the run on click. The
// ranking/status logic lives in relay-map (tested there); this locks the row's
// copy + the click.
function row(over: Partial<KitInbox> = {}): KitInbox {
	return {
		run: {
			id: 'r1',
			project: 'lumen-auth',
			assistant: '',
			state: 'running',
			task: 'rotate tokens',
			elapsed: '1m',
			edits: 3,
			last: '4m'
		},
		status: 'running',
		needs: 0,
		attention: null,
		rank: 2,
		done: 3,
		total: 10,
		...over
	};
}

afterEach(cleanup);

describe('InboxRow', () => {
	it('shows a plural needs-you why-line when needs > 1', () => {
		const { getByText } = render(InboxRow, {
			row: row({ needs: 2, attention: 'gate', rank: 0 }),
			onOpen: () => {}
		});
		expect(getByText('2 need you')).toBeTruthy();
	});

	it('shows a singular needs-you why-line when needs === 1', () => {
		const { getByText } = render(InboxRow, {
			row: row({ needs: 1, attention: 'gate', rank: 0 }),
			onOpen: () => {}
		});
		expect(getByText('1 needs you')).toBeTruthy();
	});

	it('shows the attention why-line for a stalled run', () => {
		const { getByText } = render(InboxRow, {
			row: row({ status: 'stalled', attention: 'stalled', rank: 1 }),
			onOpen: () => {}
		});
		expect(getByText('no heartbeat')).toBeTruthy();
	});

	it('falls back to the status label + shows done/total when nothing pends', () => {
		const { getByText } = render(InboxRow, { row: row(), onOpen: () => {} });
		expect(getByText('running')).toBeTruthy();
		expect(getByText('3/10')).toBeTruthy();
	});

	it('opens the run on click', () => {
		const onOpen = vi.fn();
		const { getByRole } = render(InboxRow, { row: row(), onOpen });
		getByRole('button').click();
		expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({ id: 'r1' }));
	});
});
