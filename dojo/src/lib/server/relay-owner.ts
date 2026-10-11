// Relay data belongs to the member whose machine produced it (#249).
//
// A user runs tasks on their own machine; nobody else in the dōjō sees their
// gates, segments or replies, and nobody else answers their approval gates.
// Every relay read and write is therefore scoped to the CALLER's membership —
// matching the RLS on these tables (`dojo.owns_membership`) instead of
// overriding it with the tenant's whole membership list.
import { apiError, type Caller } from './dojo-auth';

/**
 * The caller's own membership id, or a 403. A caller with no membership in this
 * dōjō has no relay data; answering with every member's would be the leak this
 * exists to prevent. Throws a Response, like the access resolvers.
 */
export function ownMembership(caller: Pick<Caller, 'membershipId'>): string {
	if (!caller.membershipId) throw apiError(403, 'not a member of this dōjō');
	return caller.membershipId;
}
