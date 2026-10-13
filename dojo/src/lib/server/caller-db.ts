// A client that reads and writes AS THE CALLER (#236, docs/spec/dojo/access-matrix.md):
// their JWT on the anon key, so the table policies decide which rows they see
// and change — not the route's filter alone. A caller with no JWT cannot act as
// themselves, and is refused rather than handed the service role.
import { apiError, type Caller } from './dojo-auth';
import { userDb } from './dojo-supabase';

export function callerDb(caller: Pick<Caller, 'accessToken'>) {
	if (!caller.accessToken) throw apiError(401, 'a signed-in session is required');
	return userDb(caller.accessToken);
}
