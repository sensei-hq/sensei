// Unit tests for the lead engagements write logic (`engagements-data.ts`).
// Exercises: parsePatchEngagement (enum + empty-body guards), updateEngagement
// (404), deleteEngagement (true/false), mergeBinding (idempotent-on-project_id,
// pure), parseBindProject, and bindEngagementProject (read → merge → write, 404).
import { describe, it, expect } from 'vitest';
import {
	parsePatchEngagement,
	updateEngagement,
	deleteEngagement,
	parseBindProject,
	mergeBinding,
	bindEngagementProject,
	EngagementsError,
	type DojoClient
} from './engagements-data';

type MutTerminal = { data?: unknown; error: unknown };

/** A single-terminal mutation stub (update/delete): captures the payload,
 *  resolves `result` on `.maybeSingle()` or on awaiting `.select()` (delete). */
function makeMutDb(result: MutTerminal) {
	const captured: { op?: string; payload?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.update = (p: unknown) => {
		captured.op = 'update';
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
	b.then = (resolve: (v: MutTerminal) => unknown) => resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('parsePatchEngagement', () => {
	it('rejects an empty body and a bad status', () => {
		expect(() => parsePatchEngagement({})).toThrow(EngagementsError);
		expect(() => parsePatchEngagement({ status: 'paused' })).toThrow();
	});
	it('accepts a close (status=ended) and description edits', () => {
		expect(parsePatchEngagement({ status: 'ended' })).toEqual({ status: 'ended' });
		expect(parsePatchEngagement({ description: 'x' })).toEqual({ description: 'x' });
	});
});

describe('updateEngagement / deleteEngagement', () => {
	it('updateEngagement 404s when nothing matched', async () => {
		const { db } = makeMutDb({ data: null, error: null });
		await expect(updateEngagement(db, 't1', 'e9', parsePatchEngagement({ status: 'ended' }))).rejects.toMatchObject({ status: 404 });
	});
	it('updateEngagement returns { id } on success', async () => {
		const { db } = makeMutDb({ data: { id: 'e1' }, error: null });
		expect(await updateEngagement(db, 't1', 'e1', parsePatchEngagement({ status: 'ended' }))).toEqual({ id: 'e1' });
	});
	it('deleteEngagement is true/false by rows removed', async () => {
		expect(await deleteEngagement(makeMutDb({ data: [{ id: 'e1' }], error: null }).db, 't1', 'e1')).toBe(true);
		expect(await deleteEngagement(makeMutDb({ data: [], error: null }).db, 't1', 'e9')).toBe(false);
	});
});

describe('mergeBinding', () => {
	it('appends a new binding to an empty/malformed array', () => {
		expect(mergeBinding(null, { project_id: 'p1', name: 'One' })).toEqual([{ project_id: 'p1', name: 'One' }]);
		expect(mergeBinding('nope', { project_id: 'p1' })).toEqual([{ project_id: 'p1' }]);
	});
	it('is idempotent on project_id — a re-bind updates the name in place', () => {
		const existing = [{ project_id: 'p1', name: 'Old' }];
		const merged = mergeBinding(existing, { project_id: 'p1', name: 'New' });
		expect(merged).toEqual([{ project_id: 'p1', name: 'New' }]);
		// pure — did not mutate the input array's row
		expect(existing[0].name).toBe('Old');
	});
	it('appends when the project_id is new', () => {
		expect(mergeBinding([{ project_id: 'p1' }], { project_id: 'p2' })).toEqual([
			{ project_id: 'p1' },
			{ project_id: 'p2' }
		]);
	});
});

describe('parseBindProject', () => {
	it('requires a project_id', () => {
		expect(() => parseBindProject({})).toThrow(EngagementsError);
		expect(parseBindProject({ project_id: 'p1', name: 'N' })).toEqual({ project_id: 'p1', name: 'N' });
	});
});

describe('bindEngagementProject', () => {
	/** A two-step stub: the first terminal (read `.maybeSingle`) resolves the
	 *  current bindings, the second resolves the update. */
	function makeBindDb(read: MutTerminal, write: MutTerminal) {
		let call = 0;
		const captured: { written?: unknown } = {};
		const b: Record<string, unknown> = {};
		b.from = () => b;
		b.select = () => b;
		b.update = (p: unknown) => {
			captured.written = p;
			return b;
		};
		b.eq = () => b;
		b.maybeSingle = () => Promise.resolve(call++ === 0 ? read : write);
		return { db: b as unknown as DojoClient, captured };
	}

	it('404s when the engagement is not found', async () => {
		const { db } = makeBindDb({ data: null, error: null }, { data: null, error: null });
		await expect(bindEngagementProject(db, 't1', 'e9', { project_id: 'p1' })).rejects.toMatchObject({ status: 404 });
	});

	it('merges the binding into the existing array and returns { id, bound: true }', async () => {
		const { db, captured } = makeBindDb(
			{ data: { project_bindings: [{ project_id: 'p0', name: 'Zero' }] }, error: null },
			{ data: { id: 'e1' }, error: null }
		);
		const out = await bindEngagementProject(db, 't1', 'e1', { project_id: 'p1', name: 'One' });
		expect(out).toEqual({ id: 'e1', bound: true });
		expect((captured.written as { project_bindings: unknown }).project_bindings).toEqual([
			{ project_id: 'p0', name: 'Zero' },
			{ project_id: 'p1', name: 'One' }
		]);
	});
});
