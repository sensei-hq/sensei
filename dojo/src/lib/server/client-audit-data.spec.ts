// Unit tests for the confidentiality-ledger read (`client-audit-data.ts`): the
// action filter, client-name enrichment, limit clamp, and fail-closed behaviour.
import { describe, it, expect } from 'vitest';
import { getClientAuditLedger, CONFIDENTIALITY_ACTIONS, AdminError, type DojoClient } from './client-audit-data';

// A table-aware stub: audit_events ends on `.limit()` (capturing the filters),
// engagements (the client-name resolver) ends on `.in()`.
function makeDb(audit: { data: unknown; error: unknown }, engagements: { data: unknown; error: unknown }) {
	const captured: { actionIn?: unknown; limit?: number } = {};
	return {
		from(table: string) {
			const b: Record<string, unknown> = {};
			b.select = () => b;
			b.eq = () => b;
			b.order = () => b;
			b.limit = (n: number) => {
				captured.limit = n;
				return Promise.resolve(audit);
			};
			b.in = (col: string, vals: unknown) => {
				if (table === 'audit_events' && col === 'action') captured.actionIn = vals;
				return table === 'engagements' ? Promise.resolve(engagements) : b;
			};
			return b;
		},
		captured
	} as unknown as DojoClient & { captured: typeof captured };
}

describe('getClientAuditLedger', () => {
	it('filters to the confidentiality actions and enriches the client name', async () => {
		const db = makeDb(
			{ data: [{ id: 1, ts: 't', action: 'publish', target: 'x', detail: null, engagement_id: 'e1' }], error: null },
			{ data: [{ id: 'e1', client_name: 'Globex' }], error: null }
		) as DojoClient & { captured: { actionIn?: unknown; limit?: number } };
		const rows = await getClientAuditLedger(db, 't1');
		expect(db.captured.actionIn).toEqual(CONFIDENTIALITY_ACTIONS);
		expect(rows[0]).toMatchObject({ action: 'publish', client_name: 'Globex' });
	});
	it('leaves client_name null for an unbound entry', async () => {
		const db = makeDb(
			{ data: [{ id: 1, ts: 't', action: 'contained', target: null, detail: null, engagement_id: null }], error: null },
			{ data: [], error: null }
		);
		const rows = await getClientAuditLedger(db, 't1');
		expect(rows[0].client_name).toBeNull();
	});
	it('clamps the limit to 1..500 (default 200)', async () => {
		const d1 = makeDb({ data: [], error: null }, { data: [], error: null }) as DojoClient & { captured: { limit?: number } };
		await getClientAuditLedger(d1, 't1');
		expect(d1.captured.limit).toBe(200);
		const d2 = makeDb({ data: [], error: null }, { data: [], error: null }) as DojoClient & { captured: { limit?: number } };
		await getClientAuditLedger(d2, 't1', 9999);
		expect(d2.captured.limit).toBe(500);
	});
	it('fails closed (500) on the audit query error', async () => {
		const db = makeDb({ data: null, error: { message: 'boom' } }, { data: [], error: null });
		await expect(getClientAuditLedger(db, 't1')).rejects.toBeInstanceOf(AdminError);
	});
});
