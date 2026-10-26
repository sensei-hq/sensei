// A failed sign-in, read from the URL Supabase sends it back on (#297).
//
// The magic-link landing guard (#329) that also lived here is kavach's now
// (`pendingLanding`, kavach 1.3.0, on by default via `kavach.handle`).

/** A failed sign-in, as Supabase reported it. */
export interface SigninError {
	code: string;
	message: string;
}

/** Supabase's wording when two dōjō logins share an email this account also has. */
const MULTIPLE_ACCOUNTS = /multiple accounts with the same email/i;

/**
 * #297. Supabase sends a failed sign-in back with the reason in the URL fragment
 * (`#error=…&error_code=…&error_description=…`), sometimes in the query. The
 * description is shown as given, except one case worded for people: automatic
 * linking refuses when more than one dōjō login carries an email this account
 * also has. Null when nothing went wrong.
 */
export function signinErrorFrom(hash: string, search: string): SigninError | null {
	for (const raw of [hash.replace(/^#/, ''), search.replace(/^\?/, '')]) {
		const p = new URLSearchParams(raw);
		const error = p.get('error');
		if (!error) continue;
		const code = p.get('error_code') || error;
		const description = p.get('error_description') || code;
		const message = MULTIPLE_ACCOUNTS.test(description)
			? 'More than one dōjō account carries an email that this account also has, so they could not be linked. Sign in with the account you use, or ask to have the unused one removed.'
			: description;
		return { code, message };
	}
	return null;
}
