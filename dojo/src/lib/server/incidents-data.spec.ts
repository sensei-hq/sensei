// Unit tests for the lead incidents store logic (`incidents-data.ts`). Exercises:
//   • shapeIncidents — worst-severity-first ordering + open_count (resolved_at
//     is null).
//   • listIncidents — the tenant filter + the shaped envelope; error → 500.
import { describe, it, expect } from 'vitest';
import {
	shapeIncidents,
	listIncidents,
	IncidentsError,
	type Incident,
	type DojoClient
} from './incidents-data';

function inc(over: Partial<Incident> = {}): Incident {
	return {
		id: 'i1',
		engagement_id: null,
		artifact_id: null,
		title: 'incident',
		description: null,
		severity: 'medium',
		status: 'open',
		owner_id: null,
		sla_due_at: null,
		resolution: null,
		opened_at: '2026-07-01T00:00:00Z',
		resolved_at: null,
		...over
	};
}

describe('shapeIncidents', () => {
	it('orders worst-severity first, then newest-opened', () => {
		const { incidents } = shapeIncidents([
			inc({ id: 'med', severity: 'medium' }),
			inc({ id: 'crit', severity: 'critical' }),
			inc({ id: 'low', severity: 'low' }),
			inc({ id: 'high-old', severity: 'high', opened_at: '2026-06-01T00:00:00Z' }),
			inc({ id: 'high-new', severity: 'high', opened_at: '2026-07-10T00:00:00Z' })
		]);
		expect(incidents.map((i) => i.id)).toEqual(['crit', 'high-new', 'high-old', 'med', 'low']);
	});
	it('counts only open incidents (resolved_at is null)', () => {
		const { open_count } = shapeIncidents([
			inc({ resolved_at: null }),
			inc({ resolved_at: '2026-07-05T00:00:00Z' }),
			inc({ resolved_at: null })
		]);
		expect(open_count).toBe(2);
	});
	it('is pure — does not mutate its input', () => {
		const input = [inc({ severity: 'low' }), inc({ severity: 'critical' })];
		const order = input.map((i) => i.severity);
		shapeIncidents(input);
		expect(input.map((i) => i.severity)).toEqual(order);
	});
});

type Terminal = { data: unknown; error: unknown };
function makeDb(result: Terminal) {
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.order = () => Promise.resolve(result);
	return b as unknown as DojoClient;
}

describe('listIncidents', () => {
	it('returns the shaped envelope', async () => {
		const db = makeDb({
			data: [inc({ id: 'a', severity: 'low' }), inc({ id: 'b', severity: 'critical', resolved_at: '2026-07-05T00:00:00Z' })],
			error: null
		});
		const { incidents, open_count } = await listIncidents(db, 't1');
		expect(incidents.map((i) => i.id)).toEqual(['b', 'a']);
		expect(open_count).toBe(1);
	});
	it('throws IncidentsError(500) on a query error', async () => {
		const db = makeDb({ data: null, error: { message: 'boom' } });
		await expect(listIncidents(db, 't1')).rejects.toThrow(IncidentsError);
	});
});
