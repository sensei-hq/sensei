// Rate-limited attempts (#311, generalised in #308 R5b). Some actions a person
// may take only so many times in a window — a failed invite accept (a token
// cannot be guessed at speed), an Azure DevOps connect step (each one reaches
// Microsoft). Counted per person and action in dojo.request_attempts (system
// work: no session reads or writes it). A failed count is an error — never a
// free pass.
import { AdminError, type DojoClient } from './admin-data';

/** `dojo.limited_action`. */
export type LimitedAction = 'invite_accept' | 'azure_connect';

const MINUTE = 60 * 1000;

/** Attempts allowed per person within the window, per action. */
export const RATE_LIMITS: Record<LimitedAction, { limit: number; windowMs: number }> = {
	/** FAILED accepts (#311). The token is unguessable; this bounds the guessing anyway. */
	invite_accept: { limit: 10, windowMs: 15 * MINUTE },
	/** Every connect-flow step (start, callback, organization list, import). */
	azure_connect: { limit: 30, windowMs: 15 * MINUTE }
};

/** How many more attempts the person has at this action in the current window
 *  (never below 0). */
export async function attemptsLeft(db: DojoClient, principalId: string, action: LimitedAction, nowMs: number): Promise<number> {
	const { limit, windowMs } = RATE_LIMITS[action];
	const since = new Date(nowMs - windowMs).toISOString();
	const { count, error } = await db
		.from('request_attempts')
		.select('principal_id', { count: 'exact', head: true })
		.eq('principal_id', principalId)
		.eq('action', action)
		.gte('at', since);
	if (error) throw new AdminError(500, error.message);
	return Math.max(0, limit - (count ?? 0));
}

/** Record one attempt by the person at this action. */
export async function recordAttempt(db: DojoClient, principalId: string, action: LimitedAction): Promise<void> {
	const { error } = await db.from('request_attempts').insert({ principal_id: principalId, action });
	if (error) throw new AdminError(500, error.message);
}

/** Refuse (429) when the person has no attempts left at this action, otherwise
 *  record this one — for an action every attempt of which counts. */
export async function guardAttempt(db: DojoClient, principalId: string, action: LimitedAction, nowMs = Date.now()): Promise<void> {
	if ((await attemptsLeft(db, principalId, action, nowMs)) === 0) {
		throw new AdminError(429, 'too many attempts — try again in a few minutes');
	}
	await recordAttempt(db, principalId, action);
}
