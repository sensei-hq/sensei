// A client that reads and writes AS THE CALLER (#236, docs/spec/dojo/access-matrix.md):
// their JWT on the anon key, so the table policies decide which rows they see
// and change — not the route's filter alone. A caller with no JWT cannot act as
// themselves, and is refused rather than handed the service role.
import { apiError, type Caller } from './dojo-auth';
import { userDb, dojoDb } from './dojo-supabase';

export function callerDb(caller: Pick<Caller, 'accessToken'>) {
	if (!caller.accessToken) throw apiError(401, 'a signed-in session is required');
	return userDb(caller.accessToken);
}

/**
 * The client for the daemon's tenant calls during the one-release window of
 * #236 step 4 (docs/plan/2026-10-12-dojo-device-plane-as-person.md): the
 * person's session when `resolveTenantCaller` verified one, the service role
 * only for the legacy device token. The credential decided which — this never
 * falls back from one to the other. Removed with the device token in 4e.
 */
export function tenantCallerDb(caller: Pick<Caller, 'accessToken'>) {
	return caller.accessToken ? callerDb(caller) : dojoDb();
}
