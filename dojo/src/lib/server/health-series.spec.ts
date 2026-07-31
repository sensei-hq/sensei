// Unit tests for the Health contributions-vs-approvals weekly series
// (`health-series.ts`): the pure bucketing and the tenant-scoped read.
import { describe, it, expect } from 'vitest';
import { bucketContribApprove, getContribVsApprove, type DojoClient } from './health-series';
import { AdminError } from './admin-data';

const NOW = new Date('2026-07-30T00:00:00Z');

describe('bucketContribApprove', () => {
	it('returns `weeks` empty W1..Wn buckets for no events', () => {
		expect(bucketContribApprove([], NOW, 4)).toEqual([
			{ wk: 'W1', c: 0, a: 0 },
			{ wk: 'W2', c: 0, a: 0 },
			{ wk: 'W3', c: 0, a: 0 },
			{ wk: 'W4', c: 0, a: 0 }
		]);
	});
	it('counts approve as an approval and publish/distribute as contributions, in the most-recent bucket', () => {
		const events = [
			{ ts: '2026-07-29T00:00:00Z', action: 'publish' }, // 1d ago → W4
			{ ts: '2026-07-29T00:00:00Z', action: 'distribute' }, // W4
			{ ts: '2026-07-29T00:00:00Z', action: 'approve' } // W4
		];
		const b = bucketContribApprove(events, NOW, 4);
		expect(b[3]).toEqual({ wk: 'W4', c: 2, a: 1 });
	});
	it('places older events in earlier buckets', () => {
		const events = [
			{ ts: '2026-07-10T00:00:00Z', action: 'publish' }, // 20d ago → weeksAgo 2 → W2
			{ ts: '2026-07-02T00:00:00Z', action: 'approve' } // 28d ago → weeksAgo 4 → outside (>=4)
		];
		const b = bucketContribApprove(events, NOW, 4);
		expect(b[1]).toEqual({ wk: 'W2', c: 1, a: 0 });
		expect(b.reduce((n, w) => n + w.a, 0)).toBe(0); // the 28d approve fell outside the window
	});
	it('ignores future-dated events', () => {
		const b = bucketContribApprove([{ ts: '2026-08-05T00:00:00Z', action: 'publish' }], NOW, 4);
		expect(b.reduce((n, w) => n + w.c, 0)).toBe(0);
	});
});

function makeDb(result: { data: unknown; error: unknown }) {
	const captured: { actionIn?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.in = (_c: string, v: unknown) => {
		captured.actionIn = v;
		return b;
	};
	b.gte = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('getContribVsApprove', () => {
	it('reads the counted actions and buckets them', async () => {
		const { db, captured } = makeDb({
			data: [
				{ ts: '2026-07-29T00:00:00Z', action: 'publish' },
				{ ts: '2026-07-29T00:00:00Z', action: 'approve' }
			],
			error: null
		});
		const series = await getContribVsApprove(db, 't1', NOW, 4);
		expect(captured.actionIn).toEqual(['publish', 'distribute', 'approve']);
		expect(series[3]).toEqual({ wk: 'W4', c: 1, a: 1 });
	});
	it('is honest-empty when there are no events', async () => {
		const { db } = makeDb({ data: [], error: null });
		const series = await getContribVsApprove(db, 't1', NOW, 4);
		expect(series.every((w) => w.c === 0 && w.a === 0)).toBe(true);
	});
	it('fails closed (500) on a query error', async () => {
		const { db } = makeDb({ data: null, error: { message: 'boom' } });
		await expect(getContribVsApprove(db, 't1', NOW)).rejects.toBeInstanceOf(AdminError);
	});
});
