import { describe, it, expect } from 'vitest';
import { readError, NOT_A_MEMBER } from './read-error';
import { DojoApiError } from './dojo-api';

// A failed read keeps its status on its way to the screen (M3, r1), so a 401/403
// can be said as a refusal rather than "Retry".
describe('readError', () => {
	it('a DojoApiError keeps its status and message', () => {
		expect(readError(new DojoApiError(403, 'admins only'))).toEqual({ status: 403, message: 'admins only' });
	});

	it('anything else is "the dōjō never answered" (status 0)', () => {
		expect(readError(new TypeError('fetch failed'))).toEqual({ status: 0, message: 'could not reach the dojo service' });
	});

	it('a membership with no tenant is a refusal, not a blank', () => {
		expect(NOT_A_MEMBER.status).toBe(403);
	});
});
