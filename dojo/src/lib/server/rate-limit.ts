// Rate-limited attempts (#311, generalised in #308 R5b). Some actions a person
// may take only so many times in a window — a failed invite accept (a token
// cannot be guessed at speed), an Azure DevOps connect step (each one reaches
// Microsoft). An attempt is TAKEN atomically by dojo.take_attempt — count and
// record in one call, serialised per person and action, old rows purged — so
// concurrent requests cannot all pass a check made before any of them records
// (#308 R5b review 2). System work; a failed call is an error, never a free pass.
import { AdminError, type DojoClient } from './admin-data';

/** `dojo.limited_action`. */
export type LimitedAction = 'invite_accept' | 'azure_connect';

const MINUTE = 60 * 1000;

/** Attempts allowed per person within the window, per action. */
export const RATE_LIMITS: Record<LimitedAction, { limit: number; windowMs: number }> = {
	/** FAILED accepts (#311): every accept takes one; one that did not fail as a
	 *  guess gives it back. The token is unguessable; this bounds the guessing. */
	invite_accept: { limit: 10, windowMs: 15 * MINUTE },
	/** Every connect-flow step (start, a matching callback, organization list, import). */
	azure_connect: { limit: 30, windowMs: 15 * MINUTE }
};

/** Take one attempt at the action: its id, or null when none are left. */
export async function takeAttempt(db: DojoClient, principalId: string, action: LimitedAction): Promise<string | null> {
	const { limit, windowMs } = RATE_LIMITS[action];
	const { data, error } = await db
		.rpc('take_attempt', {
			p_principal: principalId,
			p_action: action,
			p_limit: limit,
			p_window: `${Math.round(windowMs / 1000)} seconds`
		})
		.single();
	if (error) throw new AdminError(500, error.message);
	return typeof data === 'string' && data ? data : null;
}

/** Give an attempt back (it did not count, e.g. a successful invite accept). */
export async function releaseAttempt(db: DojoClient, attemptId: string): Promise<void> {
	const { error } = await db.rpc('release_attempt', { p_id: attemptId });
	if (error) throw new AdminError(500, error.message);
}

/** Take one attempt at an action every attempt of which counts, or refuse (429). */
export async function guardAttempt(db: DojoClient, principalId: string, action: LimitedAction): Promise<void> {
	if (!(await takeAttempt(db, principalId, action))) {
		throw new AdminError(429, 'too many attempts — try again in a few minutes');
	}
}
