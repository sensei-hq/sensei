// Unit tests for the shared audit writer (`audit.ts`). Exercises:
//   • recordAudit — the row it inserts (tenant + actor + action + target +
//     engagement_id + detail, with the right defaults).
//   • the fire-and-forget contract — a failed insert logs but does NOT throw.
import { describe, it, expect, vi } from 'vitest';
import { recordAudit, type DojoClient } from './audit';

/** A stub whose `.from(...).insert(row)` captures the row and resolves `result`. */
function makeDb(result: { error: unknown } = { error: null }) {
	const captured: { table?: string; row?: Record<string, unknown> } = {};
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		captured.table = t;
		return b;
	};
	b.insert = (row: Record<string, unknown>) => {
		captured.row = row;
		return Promise.resolve(result);
	};
	return { db: b as unknown as DojoClient, captured };
}

describe('recordAudit', () => {
	it('inserts the tenant + actor + action + defaults into audit_events', async () => {
		const { db, captured } = makeDb();
		await recordAudit(db, 't1', 'actor-1', { action: 'role_changed', target: 'u9' });
		expect(captured.table).toBe('audit_events');
		expect(captured.row).toEqual({
			tenant_id: 't1',
			actor_id: 'actor-1',
			action: 'role_changed',
			target: 'u9',
			engagement_id: null,
			detail: {}
		});
	});

	it('forwards detail + engagementId when supplied', async () => {
		const { db, captured } = makeDb();
		await recordAudit(db, 't1', 'actor-1', {
			action: 'incident_opened',
			target: 'i5',
			detail: { severity: 'high' },
			engagementId: 'e2'
		});
		expect(captured.row?.detail).toEqual({ severity: 'high' });
		expect(captured.row?.engagement_id).toBe('e2');
	});

	it('is fire-and-forget — a failed insert logs but never throws', async () => {
		const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
		const { db } = makeDb({ error: { message: 'boom' } });
		await expect(
			recordAudit(db, 't1', 'actor-1', { action: 'policy_edited', target: 'k' })
		).resolves.toBeUndefined();
		expect(spy).toHaveBeenCalledOnce();
		spy.mockRestore();
	});
});
