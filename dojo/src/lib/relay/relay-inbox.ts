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

/** What the fan-out found. A dōjō that did not answer, or a run whose plan did not
 *  load, is NAMED here rather than read as "no runs" / "no plan" (#104). */
export interface InboxLoad {
	sessions: RelaySession[];
	/** How many dōjōs were asked — so "every one failed" is distinguishable. */
	tenants: number;
	/** The tenantKeys whose relay did not answer. */
	unreachable: string[];
	/** Runs listed without their plan, because its segments read failed. */
	plansMissing: number;
}

export async function loadRelayInbox(opts: InboxLoadOpts): Promise<InboxLoad> {
	const perTenant = await Promise.all(opts.memberships.map((m) => loadTenant(m.url, opts)));
	return {
		sessions: perTenant.flatMap((t) => t.sessions),
		tenants: opts.memberships.length,
		unreachable: perTenant.filter((t) => t.failed).map((t) => t.tenantKey),
		plansMissing: perTenant.reduce((n, t) => n + t.plansMissing, 0)
	};
}

interface TenantLoad {
	tenantKey: string;
	sessions: RelaySession[];
	failed: boolean;
	plansMissing: number;
}

// One dōjō's sessions. A failure is recorded against the dōjō, so one unreachable
// dōjō neither blanks the inbox nor passes for one with nothing in flight.
async function loadTenant(tenantKey: string, opts: InboxLoadOpts): Promise<TenantLoad> {
	const call = { fetch: opts.fetch, accessToken: opts.accessToken };
	let runs: RelayRun[];
	let gates: RelayGate[];
	try {
		[runs, gates] = await Promise.all([listRuns(tenantKey, call), listGates(tenantKey, call)]);
	} catch {
		return { tenantKey, sessions: [], failed: true, plansMissing: 0 };
	}
	let plansMissing = 0;
	const sessions = await Promise.all(
		runs.map(async (run) => {
			const segments = await getSegments(tenantKey, run.run_id, call).catch(() => {
				plansMissing += 1;
				return [];
			});
			const runGates = gates.filter((g) => g.run_id === run.run_id);
			return toRelaySession(run, runGates, segments);
		})
	);
	return { tenantKey, sessions, failed: false, plansMissing };
}
