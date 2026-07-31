// Pure view-logic for the personal (solo) home — the landing a signed-in user
// with NO Dōjō membership sees (DJ1, mockup dojo-saas.jsx `DojoOrgsEmpty`).
//
// The cloud Dōjō Worker cannot see the local machine's projects or running
// tasks (those live in the desktop app's local daemon), so tonight the personal
// home is honest-empty: no fabricated project rows, no fake "needs you" cards.
// This module owns only the small derivations the presentational component needs
// (the solo identity + greeting + the library link), kept pure so it unit-tests
// without a DOM. The display-name / initials derivation mirrors the server-side
// `userProfile` in `$lib/server/dojo-orgs` (that module is server-only and can't
// be imported into a client component).

/** The minimal identity the personal home needs (from the layout's `user`). */
export interface PersonalUser {
	/** The signed-in user's stable id (Supabase auth uid) — used to resolve the
	 *  "you" chip against membership `user_id`s. Null when unauthenticated. */
	id?: string | null;
	name?: string | null;
	email?: string | null;
}

/** The resolved solo identity: a display label + avatar initials. */
export interface SoloIdentity {
	label: string;
	initials: string;
}

/** First+last initial of a display name. For an email, derive from the local
 *  part (before @) so `jerry.thomas@…` → "JT", not "JC" (jerry…com). Mirrors the
 *  server-side `initials` helper. */
function deriveInitials(nameOrEmail: string): string {
	const base = nameOrEmail.includes('@') ? nameOrEmail.split('@')[0] : nameOrEmail;
	const parts = base
		.trim()
		.split(/[\s._-]+/)
		.filter(Boolean);
	const first = parts[0]?.[0] ?? '?';
	const last = parts.length > 1 ? parts[parts.length - 1][0] : '';
	return (first + last).toUpperCase();
}

/**
 * Resolve the solo identity from the signed-in user. Magic-link users have no
 * name, so the label falls back to the email's local-part, then to "you".
 */
export function soloIdentity(user: PersonalUser | undefined): SoloIdentity {
	const name = user?.name?.trim();
	const email = user?.email?.trim();
	if (name) return { label: name, initials: deriveInitials(name) };
	if (email) return { label: email.split('@')[0], initials: deriveInitials(email) };
	return { label: 'you', initials: '?' };
}

/** The calm eyebrow line naming who's signed in and that they're working solo. */
export function personalGreeting(user: PersonalUser | undefined): string {
	return `signed in as ${soloIdentity(user).label} · working solo`;
}
