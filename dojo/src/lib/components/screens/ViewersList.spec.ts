import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ViewersList from './ViewersList.svelte';
import type { DojoViewerView } from '$lib/viewers-view';

// The dōjō's viewers (#307 A3, D8): each viewer's name (or invited email), the
// workspaces granted to them with a revoke, a picker of the workspaces not yet
// granted, and remove. Nothing is fabricated: no viewers says so.

const viewers: DojoViewerView[] = [
	{
		principalId: 'boss',
		name: 'The Boss',
		email: 'boss@example.com',
		workspaces: [{ id: 'p1', name: 'Payments' }]
	},
	{ principalId: 'client', name: null, email: 'client@example.com', workspaces: [] }
];
const workspaces = [
	{ id: 'p1', name: 'Payments' },
	{ id: 'p2', name: 'Search' }
];

describe('ViewersList', () => {
	afterEach(cleanup);

	it('names each viewer — by email when there is no name — with their workspaces', () => {
		const { getByText, getAllByText } = render(ViewersList, { props: { viewers, workspaces } });
		expect(getByText('The Boss')).toBeTruthy();
		expect(getAllByText('client@example.com').length).toBeGreaterThan(0);
		expect(getByText('Payments')).toBeTruthy();
		expect(getByText('No workspaces yet')).toBeTruthy();
	});

	it('grants a workspace not yet granted, revokes one, and removes a viewer', async () => {
		const onGrant = vi.fn();
		const onRevoke = vi.fn();
		const onRemove = vi.fn();
		const { getByLabelText, getByRole } = render(ViewersList, {
			props: { viewers, workspaces, onGrant, onRevoke, onRemove }
		});
		// The Boss already has Payments: only Search is offered.
		const pick = getByLabelText('Grant a workspace to The Boss') as HTMLSelectElement;
		expect([...pick.options].map((o) => o.value)).toEqual(['', 'p2']);
		await fireEvent.change(pick, { target: { value: 'p2' } });
		expect(onGrant).toHaveBeenCalledWith(viewers[0], 'p2');
		await fireEvent.click(getByRole('button', { name: 'Revoke Payments from The Boss' }));
		expect(onRevoke).toHaveBeenCalledWith(viewers[0], 'p1');
		await fireEvent.click(getByRole('button', { name: 'Remove client@example.com' }));
		expect(onRemove).toHaveBeenCalledWith(viewers[1]);
	});

	it('says when there are no viewers', () => {
		const { getByText } = render(ViewersList, { props: { viewers: [], workspaces } });
		expect(getByText(/No viewers yet/)).toBeTruthy();
	});
});
