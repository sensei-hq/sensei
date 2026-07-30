// Load seam (layer 3 of ui-state-pattern). REAL user-wide read: the relay API is
// single-tenant, so fan out over EVERY membership (each tenantKey = the org's
// discovery path, org.url) and merge — the personal inbox spans all the user's dōjōs
// (entity-access-model.md). Per-run segments feed the rail pips + the detail plan.
// The mock stays in relay-inbox.mock.ts as the test fixture. The membership-derived
// RLS single-query is a later backend optimization (see the relay RLS design doc).
import type { RelaySession } from './types';
import { listRuns, listGates, getSegments, type RelayRun, type RelayGate } from '$lib/relay-data';
import { toRelaySession } from './relay-inbox-map';

export interface InboxLoadOpts {
	/** The user's memberships — each `.url` is its tenantKey (discovery path). */
	memberships: { url: string }[];
	accessToken: string | null;
	fetch: typeof fetch;
}

export async function loadRelayInbox(opts: InboxLoadOpts): Promise<RelaySession[]> {
	const perTenant = await Promise.all(opts.memberships.map((m) => loadTenant(m.url, opts)));
	return perTenant.flat();
}

// One dōjō's sessions. A single tenant's failure yields [] so one unreachable dōjō
// never blanks the whole inbox.
async function loadTenant(tenantKey: string, opts: InboxLoadOpts): Promise<RelaySession[]> {
	const call = { fetch: opts.fetch, accessToken: opts.accessToken };
	try {
		const [runs, gates]: [RelayRun[], RelayGate[]] = await Promise.all([
			listRuns(tenantKey, call),
			listGates(tenantKey, call)
		]);
		return Promise.all(
			runs.map(async (run) => {
				const segments = await getSegments(tenantKey, run.run_id, call).catch(() => []);
				const runGates = gates.filter((g) => g.run_id === run.run_id);
				return toRelaySession(run, runGates, segments);
			})
		);
	} catch {
		return [];
	}
}
