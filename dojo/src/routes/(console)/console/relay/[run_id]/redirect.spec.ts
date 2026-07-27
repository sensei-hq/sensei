import { describe, it, expect } from 'vitest';
import { load } from './+page';

// The legacy /console/relay/<id> deep link (old push notifications) must land on
// the new /you/runs/<id> run detail — a permanent (308) redirect.
describe('/console/relay/[run_id] → /you/runs/[run_id]', () => {
	it('permanently redirects to the run detail under /you', () => {
		try {
			load({ params: { run_id: 'run-42' } } as unknown as Parameters<typeof load>[0]);
			throw new Error('expected the load to throw a redirect');
		} catch (e) {
			const r = e as { status?: number; location?: string };
			expect(r.status).toBe(308);
			expect(r.location).toBe('/you/runs/run-42');
		}
	});
});
