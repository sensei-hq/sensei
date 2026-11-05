// The dōjō Delivery page's selection (tracker-delivery-metrics D5; #352): a sprint's
// tickets read on demand, a failed read an error, a slower earlier answer dropped.
import { describe, it, expect } from 'vitest';
import { DeliveryPageState } from './delivery-state.svelte';
import type { DeliveryTicketWire } from './workspace-delivery-data';

const t = (key: string, title: string | null = `${key} title`): DeliveryTicketWire => ({
	key,
	title,
	status_category: null,
	points: null,
	sprints: [],
	issue_created_at: null,
	team: null,
	team_words: null,
	contributors: 0,
	last_report: null
});

const nobody = { line: null, figures: [] };

describe('DeliveryPageState', () => {
	it('selecting a sprint reads its tickets, titled; a ticket with no title shows its key alone', async () => {
		const asked: string[] = [];
		const s = new DeliveryPageState(async (board, sprint) => {
			asked.push(`${board}/${sprint}`);
			return [t('AUTH-2', 'Two'), t('AUTH-3', null)];
		});
		await s.select('b1', '101');
		expect(asked).toEqual(['b1/101']);
		expect(s.selectedFor('b1')).toBe('101');
		expect(s.tickets).toEqual([
			{ key: 'AUTH-2', title: 'Two', team: nobody },
			{ key: 'AUTH-3', title: null, team: nobody }
		]);
		expect(s.ticketsError).toBe(null);
	});

	it('a failed read is an error, never an empty list', async () => {
		const s = new DeliveryPageState(async () => {
			throw new Error('request failed (500)');
		});
		await s.select('b1', '101');
		expect(s.tickets).toBe(null);
		expect(s.ticketsError).toBe('request failed (500)');
	});

	it('a slower earlier selection is dropped', async () => {
		let release!: () => void;
		const gate = new Promise<void>((r) => (release = r));
		const s = new DeliveryPageState(async (_b, sprint) => {
			if (sprint === 'old') await gate;
			return [t(sprint === 'old' ? 'OLD-1' : 'NEW-1')];
		});
		const first = s.select('b1', 'old');
		await s.select('b1', 'new');
		release();
		await first;
		expect(s.selectedFor('b1')).toBe('new');
		expect(s.tickets?.map((x) => x.key)).toEqual(['NEW-1']);
	});
});
