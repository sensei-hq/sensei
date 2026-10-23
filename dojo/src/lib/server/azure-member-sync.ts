// The scheduled Azure DevOps member read (#308 R5b, D44).
//
// Besides import and Sync, a cron run reads each connected Azure organization's
// members and applies #309's rule: a COMPLETE read ends the memberships its
// organization provisioned for people it no longer lists; a partial, empty or
// failed read ends nobody (followMembers, reconcile_unit_members).
//
//   * Only connections whose grant still works (token_state ok); a grant
//     Microsoft refuses is marked "reconnect" (accessFromStoredGrant) and skipped
//     from then on.
//   * Bounded: at most MEMBER_SYNC_BATCH connections per run, the ones read
//     longest ago (or never) first — a Worker invocation has a subrequest budget.
//   * One connection's failure never stops the others, and is RECORDED on the
//     connection (members_read_at, members_read_error) and in the run's answer —
//     never swallowed. No token appears in either.
// System work (the service-role client). Triggered by the Worker's cron
// (src/worker-entry.js) through /v1/internal/azure/member-sync.
import { AdminError, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';
import { accessFromStoredGrant, followMembers, proveGrant, type AzureMembersRead, type AzureStoredGrantDeps } from './azure-import';
import type { AzureAdapter } from './forge-azure';

/** Connections read per run. */
export const MEMBER_SYNC_BATCH = 10;

export type MemberSyncResult = { organization: string; members: AzureMembersRead } | { organization: string; error: string };

export interface MemberSyncRun {
	read: number;
	results: MemberSyncResult[];
}

interface ConnRow {
	tenant_id: string;
	external_id: string;
	external_slug: string;
}

/** Record how one connection's read went: when, and why it failed (or null). */
async function record(db: DojoClient, orgId: string, nowIso: string, error: string | null): Promise<void> {
	const { error: failed } = await db
		.from('tenant_connections')
		.update({ members_read_at: nowIso, members_read_error: error })
		.eq('provider', 'azure_devops')
		.eq('external_id', orgId);
	if (failed) throw new AdminError(refusalStatus(failed), failed.message);
}

/** Read one connection's members with its stored grant and apply #309's rule. */
async function readOne(db: DojoClient, c: ConnRow, deps: MemberSyncDeps): Promise<MemberSyncResult> {
	const token = await accessFromStoredGrant(db, c.external_id, deps);
	const adapter = deps.adapterFor(c.external_slug);
	// The same proof Sync and import run (proveGrant): a grant whose owner is no
	// longer a Project Collection Administrator throws here — recorded, nobody ended.
	const { conn } = await proveGrant(adapter, token);
	if (conn.orgId !== c.external_id) {
		return { organization: c.external_slug, error: `${c.external_slug} now names a different Azure DevOps organization; nobody was ended` };
	}
	const members = await followMembers(db, c.tenant_id, adapter, token, { id: c.external_id, login: c.external_slug });
	return { organization: c.external_slug, members };
}

export interface MemberSyncDeps extends AzureStoredGrantDeps {
	adapterFor: (organization: string) => AzureAdapter;
	now?: number;
	batch?: number;
}

/** One scheduled run. Throws only when the connections cannot be read at all. */
export async function runAzureMemberSync(db: DojoClient, deps: MemberSyncDeps): Promise<MemberSyncRun> {
	const nowIso = new Date(deps.now ?? Date.now()).toISOString();
	const batch = deps.batch ?? MEMBER_SYNC_BATCH;
	const { data, error } = await db
		.from('tenant_connections')
		.select('tenant_id, external_id, external_slug')
		.eq('provider', 'azure_devops')
		.eq('token_state', 'ok')
		.order('members_read_at', { ascending: true, nullsFirst: true })
		.range(0, batch - 1);
	if (error) throw new AdminError(500, error.message);

	const results: MemberSyncResult[] = [];
	const unrecorded: string[] = [];
	for (const c of (data ?? []) as ConnRow[]) {
		let result: MemberSyncResult;
		try {
			result = await readOne(db, c, deps);
		} catch (e) {
			result = { organization: c.external_slug, error: e instanceof Error ? e.message : 'the member read failed' };
		}
		results.push(result);
		const failure = 'error' in result ? result.error : 'error' in result.members ? result.members.error : null;
		if (failure) console.error(`azure member sync: ${c.external_slug}: ${failure}`);
		// A record that fails must not stop the run: the next connections still
		// get read (their reads are what advance the batch). The run fails at the
		// end instead, so it shows in Cron Events (#308 R5b review 2).
		try {
			await record(db, c.external_id, nowIso, failure);
		} catch (e) {
			const why = e instanceof Error ? e.message : String(e);
			console.error(`azure member sync: could not record ${c.external_slug}: ${why}`);
			unrecorded.push(`${c.external_slug} (${why})`);
		}
	}
	if (unrecorded.length) {
		throw new AdminError(500, `could not record ${unrecorded.length} member reads: ${unrecorded.join('; ')}`);
	}
	return { read: results.length, results };
}
