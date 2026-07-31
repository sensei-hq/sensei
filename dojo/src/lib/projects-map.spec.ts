// Unit tests for the org Projects wire→kit mapper (`projects-map.ts`).
import { describe, expect, it } from 'vitest';
import { toKitProject, toKitProjects } from './projects-map';
import type { ProjectRow } from './client-data';

const NOW = new Date('2026-07-30T00:00:00Z');

function row(over: Partial<ProjectRow> = {}): ProjectRow {
	return {
		id: 'p1',
		slug: 'acme/ledger',
		name: 'ledger-core',
		classification: 'company',
		phase: 'notice',
		last_run_at: '2026-07-23T00:00:00Z',
		runs_week: 14,
		...over
	};
}

describe('toKitProject', () => {
	it('maps the row onto the kit project', () => {
		const k = toKitProject(row(), NOW);
		expect(k).toMatchObject({
			id: 'p1',
			name: 'ledger-core',
			repo: 'acme/ledger',
			classification: 'company',
			phase: 'notice',
			runsWeek: 14,
			needs: 0, // no needs-attention signal on dojo.projects yet (honest default)
			dojoName: null,
			note: ''
		});
		expect(k.lastRun).toMatch(/^7d$|d$/); // relative age of the last run
	});
	it('reads "—" for a project that has never run', () => {
		expect(toKitProject(row({ last_run_at: null }), NOW).lastRun).toBe('—');
	});
	it('preserves order', () => {
		const rows = toKitProjects([row({ id: 'a' }), row({ id: 'b' })], NOW);
		expect(rows.map((r) => r.id)).toEqual(['a', 'b']);
	});
});
