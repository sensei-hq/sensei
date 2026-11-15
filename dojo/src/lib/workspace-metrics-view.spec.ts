// The workspace metrics view: per-repository cards from the GET
// …/workspaces/{id}/metrics wire (WP5). One card per Teams-view metric per
// repository, so a workspace's numbers read like its teams' and its people's.
//
// The shaping rules, each pinned below: a day with a pooled row reads the
// pool (never pool + detail — a maintainer who also views would count twice);
// a day with user detail but no pool reads the detail; repo rows fill days
// with no user rows and are ignored beside them (the daemon's whole-tree
// twin, never additive); counts sum, rates pool by denominators; a metric
// with no rows is an empty card, never a zero.
import { describe, it, expect } from 'vitest';
import { toWorkspaceMetricsView } from './workspace-metrics-view';
import type { WorkspaceMetricsWire, WorkspaceMetricPoint } from './workspace-metrics-data';

function row(over: Partial<WorkspaceMetricPoint> = {}): WorkspaceMetricPoint {
	return {
		repository_id: 'r1',
		repo_key: 'github.com/acme/api',
		metric_key: 'throughput',
		scope: 'repo',
		principal_id: null,
		day: '2026-10-20',
		value: 4,
		props: {},
		...over
	};
}

function wire(rows: WorkspaceMetricPoint[], days = 30, since = '2026-10-05'): WorkspaceMetricsWire {
	return { since, days, rows };
}

describe('toWorkspaceMetricsView', () => {
	it('shapes one repository’s cards: counts sum, rates pool by denominators, oldest first', () => {
		const view = toWorkspaceMetricsView(
			wire([
				row({ metric_key: 'throughput', day: '2026-10-21', value: 6 }),
				row({ metric_key: 'throughput', day: '2026-10-20', value: 4 }),
				row({
					metric_key: 'ftr',
					scope: 'user',
					principal_id: 'p1',
					day: '2026-10-21',
					value: 1.0,
					props: { denominator: '10' }
				}),
				row({
					metric_key: 'ftr',
					scope: 'user',
					principal_id: 'p1',
					day: '2026-10-20',
					value: 0.5,
					props: { denominator: 10 }
				})
			])
		);
		expect(view.days).toBe(30);
		expect(view.since).toBe('2026-10-05');
		expect(view.repos).toHaveLength(1);
		const [repo] = view.repos;
		expect(repo.repository_id).toBe('r1');
		expect(repo.repo_key).toBe('github.com/acme/api');
		expect(repo.name).toBe('acme/api');
		const total = repo.cards.find((c) => c.key === 'throughput')!;
		expect(total.label).toBe('Sessions');
		expect(total.value).toBe('10');
		expect(total.points).toEqual([4, 6]);
		expect(total.empty).toBe(false);
		const rate = repo.cards.find((c) => c.key === 'ftr')!;
		expect(rate.value).toBe('75%');
		expect(rate.points).toEqual([0.5, 1.0]);
		expect(rate.empty).toBe(false);
	});

	it('splits per repository, sorted by key', () => {
		const view = toWorkspaceMetricsView(
			wire([
				row({ repository_id: 'r2', repo_key: 'github.com/acme/web', value: 2 }),
				row({ repository_id: 'r1', repo_key: 'github.com/acme/api', value: 4 })
			])
		);
		expect(view.repos.map((r) => r.name)).toEqual(['acme/api', 'acme/web']);
		expect(view.repos[0].cards.find((c) => c.key === 'throughput')?.value).toBe('4');
		expect(view.repos[1].cards.find((c) => c.key === 'throughput')?.value).toBe('2');
	});

	it('a maintainer who is also a viewer is not double-counted: a pooled day reads the pool', () => {
		// Dave maintains R and views P: the same day arrives as per-person
		// detail AND as the pool over the same people. The pool wins; the
		// total is 3, never 3 + (1 + 2).
		const view = toWorkspaceMetricsView(
			wire([
				row({ scope: 'user', principal_id: 'p1', value: 1, props: { denominator: 1 } }),
				row({ scope: 'user', principal_id: 'p2', value: 2, props: { denominator: 1 } }),
				row({ scope: 'pooled', value: 3, props: { pooled_rows: 2, numerator: 3, denominator: 2 } })
			])
		);
		const total = view.repos[0].cards.find((c) => c.key === 'throughput')!;
		expect(total.value).toBe('3');
		expect(total.points).toEqual([3]);
	});

	it('pools a rate from the pools’ numerators/denominators across days', () => {
		const view = toWorkspaceMetricsView(
			wire([
				row({
					metric_key: 'ftr',
					scope: 'pooled',
					day: '2026-10-21',
					value: 7,
					props: { pooled_rows: 3, contributors: 3, numerator: 15, denominator: 20 }
				}),
				row({
					metric_key: 'ftr',
					scope: 'pooled',
					day: '2026-10-20',
					value: 9,
					props: { pooled_rows: 2, contributors: 2, numerator: 9, denominator: 12 }
				})
			])
		);
		const rate = view.repos[0].cards.find((c) => c.key === 'ftr')!;
		// (15 + 9) / (20 + 12); each day's `value` sum ignored.
		expect(rate.value).toBe('75%');
		expect(rate.points).toEqual([0.75, 0.75]);
	});

	it('repo rows fill days with no user rows, and are ignored beside them (twins)', () => {
		// The daemon writes the whole-tree twin beside the attributed rows;
		// the two measure the same day and must never add.
		const view = toWorkspaceMetricsView(
			wire([
				row({ day: '2026-10-20', scope: 'repo', value: 5 }),
				row({ day: '2026-10-20', scope: 'user', principal_id: 'p1', value: 5 }),
				row({ day: '2026-10-21', scope: 'repo', value: 3 })
			])
		);
		const total = view.repos[0].cards.find((c) => c.key === 'throughput')!;
		expect(total.value).toBe('8');
		expect(total.points).toEqual([5, 3]);
	});

	it('a rate no day can pool is a dash, not a zero', () => {
		const view = toWorkspaceMetricsView(
			wire([
				row({ metric_key: 'ftr', scope: 'user', principal_id: 'p1', value: 0.5, props: {} }),
				row({
					metric_key: 'ftr',
					scope: 'user',
					principal_id: 'p2',
					day: '2026-10-21',
					value: 0.5,
					props: { denominator: 0 }
				})
			])
		);
		const rate = view.repos[0].cards.find((c) => c.key === 'ftr')!;
		expect(rate.value).toBe('—');
		expect(rate.points).toEqual([]);
		expect(rate.empty).toBe(false);
	});

	it('a metric with no rows is an empty card; unknown keys are not shown', () => {
		const view = toWorkspaceMetricsView(wire([row({ metric_key: 'churn_rate', value: 9 })]));
		const [repo] = view.repos;
		expect(repo.cards.map((c) => c.key)).toEqual(['throughput', 'ftr', 'rework_ratio', 'tokens_per_day']);
		const total = repo.cards.find((c) => c.key === 'throughput')!;
		expect(total.empty).toBe(true);
		expect(total.value).toBe('—');
		expect(total.points).toEqual([]);
	});

	it('an empty window is no repositories, not an error', () => {
		const view = toWorkspaceMetricsView(wire([]));
		expect(view.days).toBe(30);
		expect(view.repos).toEqual([]);
	});
});
