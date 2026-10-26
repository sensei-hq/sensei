import { describe, expect, it } from 'vitest';
import { signinErrorFrom } from './signin-error';

// #297. Supabase reports a failed sign-in in the URL fragment (`#error=…&
// error_description=…`), sometimes in the query. The sign-in page used to ignore
// it, so a failed sign-in looked like nothing happened.
describe('signinErrorFrom', () => {
	it('reads the description from the fragment, decoded', () => {
		expect(
			signinErrorFrom('#error=access_denied&error_code=otp_expired&error_description=Email+link+is+invalid+or+has+expired', '')
		).toEqual({ code: 'otp_expired', message: 'Email link is invalid or has expired' });
	});

	it('reads the query form too, and falls back to the error code when there is no description', () => {
		expect(signinErrorFrom('', '?error=server_error')).toEqual({ code: 'server_error', message: 'server_error' });
	});

	it('explains Supabase’s refusal to link two accounts that share an email', () => {
		const e = signinErrorFrom(
			'#error=server_error&error_code=unexpected_failure&error_description=Multiple+accounts+with+the+same+email+address+in+the+same+linking+domain+detected',
			''
		);
		expect(e?.code).toBe('unexpected_failure');
		expect(e?.message).toMatch(/more than one dōjō account/i);
	});

	it('is null when nothing went wrong, including a successful landing', () => {
		expect(signinErrorFrom('', '')).toBeNull();
		expect(signinErrorFrom('#access_token=x&type=magiclink', '?next=/you')).toBeNull();
	});
});
