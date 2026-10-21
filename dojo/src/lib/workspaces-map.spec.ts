// Unit tests for the org Workspaces wire→kit mapper (`workspaces-map.ts`).
import { describe, expect, it } from 'vitest';
import { toKitWorkspace, toKitWorkspaces } from './workspaces-map';
import type { WorkspaceRow } from './client-data';

const NOW = new Date('2026-07-30T00:00:00Z');

function row(over: Partial<WorkspaceRow> = {}): WorkspaceRow {
	return {
		id: 'p1',
		tenant_id: 't1',
		slug: 'acme/ledger',
		name: 'ledger-core',
		classification: 'company',
		phase: 'notice',
		last_run_at: '2026-07-23T00:00:00Z',
		runs_week: 14,
		...over
	};
}

describe('toKitWorkspace', () => {
	it('maps the row onto the kit workspace', () => {
		const k = toKitWorkspace(row(), NOW);
		expect(k).toMatchObject({
			id: 'p1',
			name: 'ledger-core',
			repo: 'acme/ledger',
			classification: 'company',
			phase: 'notice',
			runsWeek: 14,
			needs: 0, // no needs-attention signal on dojo.workspaces yet (honest default)
			dojoName: null,
			note: ''
		});
		expect(k.lastRun).toMatch(/^7d$|d$/); // relative age of the last run
	});
	it('reads "—" for a workspace that has never run', () => {
		expect(toKitWorkspace(row({ last_run_at: null }), NOW).lastRun).toBe('—');
	});
	it('preserves order', () => {
		const rows = toKitWorkspaces([row({ id: 'a' }), row({ id: 'b' })], NOW);
		expect(rows.map((r) => r.id)).toEqual(['a', 'b']);
	});
});
