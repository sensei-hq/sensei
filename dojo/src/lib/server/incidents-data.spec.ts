// Unit tests for the lead incidents store logic (`incidents-data.ts`). Exercises:
//   • shapeIncidents — worst-severity-first ordering + open_count (resolved_at
//     is null).
//   • listIncidents — the tenant filter + the shaped envelope; error → 500.
import { describe, it, expect } from 'vitest';
import {
	shapeIncidents,
	listIncidents,
	getIncidentDetail,
	createIncident,
	parseNewIncident,
	updateIncident,
	parsePatchIncident,
	deleteIncident,
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

// ── getIncidentDetail: incident + resolved client/owner names + linked artifact ──
// A table-aware stub: each `.from(table)` returns a FRESH builder (so the parallel
// resolver/artifact queries don't race on shared state); the terminal (`.in()` for
// the resolvers, `.maybeSingle()` for incident/artifact) resolves that table's result.
function makeDetailDb(tables: Record<string, { data: unknown; error: unknown }>) {
	return {
		from(table: string) {
			const res = tables[table] ?? { data: null, error: null };
			const b: Record<string, unknown> = {};
			b.select = () => b;
			b.eq = () => b;
			b.in = () => Promise.resolve(res);
			b.maybeSingle = () => Promise.resolve(res);
			return b;
		}
	} as unknown as DojoClient;
}

describe('getIncidentDetail', () => {
	it('composes the incident with resolved client name, owner name/email, and linked artifact', async () => {
		const db = makeDetailDb({
			incidents: { data: inc({ id: 'i1', engagement_id: 'e1', owner_id: 'u1', artifact_id: 'a1' }), error: null },
			engagements: { data: [{ id: 'e1', client_name: 'Globex' }], error: null },
			identities: { data: [{ user_id: 'u1', display_name: 'Ada', email: 'ada@x.co', last_login_at: null }], error: null },
			artifacts: { data: { id: 'a1', title: 'the pattern', kind: 'pattern', status: 'archived' }, error: null }
		});
		const d = await getIncidentDetail(db, 't1', 'i1');
		expect(d).toMatchObject({
			id: 'i1',
			client_name: 'Globex',
			owner_name: 'Ada',
			owner_email: 'ada@x.co',
			artifact: { id: 'a1', title: 'the pattern', kind: 'pattern', status: 'archived' }
		});
	});
	it('leaves client/owner/artifact null when the incident references none (skips those reads)', async () => {
		const db = makeDetailDb({ incidents: { data: inc({ id: 'i1' }), error: null } });
		const d = await getIncidentDetail(db, 't1', 'i1');
		expect(d.client_name).toBeNull();
		expect(d.owner_name).toBeNull();
		expect(d.owner_email).toBeNull();
		expect(d.artifact).toBeNull();
	});
	it('404s when no tenant incident matches', async () => {
		const db = makeDetailDb({ incidents: { data: null, error: null } });
		await expect(getIncidentDetail(db, 't1', 'ghost')).rejects.toMatchObject({ status: 404 });
	});
	it('fails closed (500) on the incident query error', async () => {
		const db = makeDetailDb({ incidents: { data: null, error: { message: 'boom' } } });
		await expect(getIncidentDetail(db, 't1', 'i1')).rejects.toMatchObject({ status: 500 });
	});
	it('fails closed (500) on the linked-artifact query error', async () => {
		const db = makeDetailDb({
			incidents: { data: inc({ id: 'i1', artifact_id: 'a1' }), error: null },
			artifacts: { data: null, error: { message: 'artifact boom' } }
		});
		await expect(getIncidentDetail(db, 't1', 'i1')).rejects.toMatchObject({ status: 500 });
	});
});

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

// ── writes ───────────────────────────────────────────────────────────────────
type MutTerminal = { data?: unknown; error: unknown };
function makeMutDb(result: MutTerminal) {
	const captured: { op?: string; payload?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.update = (p: unknown) => {
		captured.op = 'update';
		captured.payload = p;
		return b;
	};
	b.insert = (p: unknown) => {
		captured.op = 'insert';
		captured.payload = p;
		return b;
	};
	b.delete = () => {
		captured.op = 'delete';
		return b;
	};
	b.eq = () => b;
	b.select = () => b;
	b.maybeSingle = () => Promise.resolve(result);
	b.single = () => Promise.resolve(result);
	b.then = (resolve: (v: MutTerminal) => unknown) => resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('parseNewIncident', () => {
	it('requires a title', () => {
		expect(() => parseNewIncident({})).toThrow(IncidentsError);
	});
	it('defaults severity to medium and rejects a bad severity', () => {
		expect(parseNewIncident({ title: 'x' }).severity).toBe('medium');
		expect(parseNewIncident({ title: 'x', severity: 'critical' }).severity).toBe('critical');
		expect(() => parseNewIncident({ title: 'x', severity: 'ultra' })).toThrow();
	});
});

describe('createIncident', () => {
	it('inserts the incident and returns { id, severity }', async () => {
		const { db, captured } = makeMutDb({ data: { id: 'i1', severity: 'high' }, error: null });
		const out = await createIncident(db, 't1', parseNewIncident({ title: 'leak', severity: 'high' }));
		expect(out).toEqual({ id: 'i1', severity: 'high' });
		expect((captured.payload as { title: string }).title).toBe('leak');
	});
});

describe('parsePatchIncident', () => {
	it('rejects an empty body and bad enums', () => {
		expect(() => parsePatchIncident({})).toThrow(IncidentsError);
		expect(() => parsePatchIncident({ severity: 'x' })).toThrow();
		expect(() => parsePatchIncident({ status: 'x' })).toThrow();
	});
	it('resolves on status=resolved or resolved:true; reopens on open/investigating', () => {
		expect(parsePatchIncident({ status: 'resolved' })).toMatchObject({ resolve: true });
		expect(parsePatchIncident({ resolved: true })).toMatchObject({ resolve: true, status: 'resolved' });
		expect(parsePatchIncident({ status: 'investigating' })).toMatchObject({ reopen: true });
	});
});

describe('updateIncident', () => {
	it('stamps resolved_at when resolving', async () => {
		const { db, captured } = makeMutDb({ data: { id: 'i1' }, error: null });
		await updateIncident(db, 't1', 'i1', parsePatchIncident({ resolved: true }));
		expect((captured.payload as { resolved_at: unknown }).resolved_at).toBeTypeOf('string');
	});
	it('clears resolved_at when reopening', async () => {
		const { db, captured } = makeMutDb({ data: { id: 'i1' }, error: null });
		await updateIncident(db, 't1', 'i1', parsePatchIncident({ status: 'open' }));
		expect((captured.payload as { resolved_at: unknown }).resolved_at).toBeNull();
	});
	it('404s when nothing matched', async () => {
		const { db } = makeMutDb({ data: null, error: null });
		await expect(updateIncident(db, 't1', 'i9', parsePatchIncident({ severity: 'low' }))).rejects.toMatchObject({ status: 404 });
	});
});

describe('deleteIncident', () => {
	it('is true when a row was removed, false when none', async () => {
		expect(await deleteIncident(makeMutDb({ data: [{ id: 'i1' }], error: null }).db, 't1', 'i1')).toBe(true);
		expect(await deleteIncident(makeMutDb({ data: [], error: null }).db, 't1', 'i9')).toBe(false);
	});
});
