import { describe, it, expect, vi } from 'vitest';
import { TeamDraft } from './teams-state.svelte';

// Naming a team (#307 A3): the new-team and rename forms share one draft. A blank
// name is not sent; the write's own error is the screen's (OrgSectionState).
describe('TeamDraft', () => {
	it('creates with the typed name and closes', async () => {
		const save = vi.fn().mockResolvedValue(true);
		const d = new TeamDraft();
		d.start();
		d.name = 'Platform';
		await d.submit(save);
		expect(save).toHaveBeenCalledWith(null, 'Platform');
		expect(d.open).toBe(false);
	});

	it('renames the team it was started on', async () => {
		const save = vi.fn().mockResolvedValue(true);
		const d = new TeamDraft();
		d.start({ id: 't1', name: 'Payments' });
		expect(d.name).toBe('Payments');
		d.name = 'Payments & Billing';
		await d.submit(save);
		expect(save).toHaveBeenCalledWith('t1', 'Payments & Billing');
	});

	it('does not send a blank name, and stays open when the write failed', async () => {
		const save = vi.fn().mockResolvedValue(false);
		const d = new TeamDraft();
		d.start();
		d.name = '   ';
		await d.submit(save);
		expect(save).not.toHaveBeenCalled();
		d.name = 'Taken';
		await d.submit(save);
		expect(d.open).toBe(true);
	});
});
