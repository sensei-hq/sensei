// A read that failed, as a loader hands it to its screen: the HTTP status and
// the dōjō's message — not the message alone, so the screen can tell a refusal
// (401/403) from a fault (dojoLoadNotice → noticeFromApiError).
import { DojoApiError } from './dojo-api';

export interface ReadError {
	status: number;
	message: string;
}

/** Any thrown read failure → a ReadError. Not a DojoApiError: the dōjō never
 *  answered (status 0). */
export function readError(e: unknown): ReadError {
	if (e instanceof DojoApiError) return { status: e.status, message: e.message };
	return { status: 0, message: 'could not reach the dojo service' };
}

/** A section read for a membership with no tenant: a refusal, not a blank. */
export const NOT_A_MEMBER: ReadError = { status: 403, message: 'You are not a member of this dōjō.' };
