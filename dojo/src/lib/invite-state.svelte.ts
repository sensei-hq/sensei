// The admin invite (#307 A3, D8). A member invite makes a membership at a role;
// a viewer invite makes a dōjō viewer — no role, no repositories — whom the admin
// then grants selected projects. The dōjō issues a single-use token; the admin
// shares the accept link (there is no email delivery yet).
import { DojoApiError } from '$lib/client-data';
import { looksLikeEmail, type MEMBERSHIP_KINDS } from '$lib/admin-view';

export type InviteAccess = 'member' | 'viewer';
export type InviteKind = (typeof MEMBERSHIP_KINDS)[number];

/** What the admin is filling in. `role` is ignored for a viewer invite. */
export interface InviteDraft {
	email: string;
	access: InviteAccess;
	role: string;
	kind: InviteKind;
}

/** The body `POST …/invites` takes. */
export interface InviteInput {
	email: string;
	kind: InviteKind;
	access: InviteAccess;
	role?: string;
}

/** What the dōjō answers with — the token goes into the accept link. */
export interface IssuedInviteWire {
	token: string;
	email: string;
}

/** The invite the admin just issued, ready to share. */
export interface IssuedInvite {
	email: string;
	access: InviteAccess;
	link: string;
}

export const emptyDraft = (): InviteDraft => ({ email: '', access: 'member', role: 'contributor', kind: 'employer' });

/** Why the draft cannot be sent, or null when it can. */
export function draftProblem(d: InviteDraft): string | null {
	const email = d.email.trim();
	if (!email) return 'Enter the email address to invite.';
	if (!looksLikeEmail(email)) return 'That is not an email address.';
	return null;
}

/** The request body. A viewer invite carries no role: a viewer is not a member. */
export function toInviteInput(d: InviteDraft): InviteInput {
	const base = { email: d.email.trim().toLowerCase(), kind: d.kind, access: d.access };
	return d.access === 'viewer' ? base : { ...base, role: d.role };
}

/** The accept link the invitee opens (signed in as the invited email). */
export function inviteLink(origin: string, token: string): string {
	return `${origin}/you/invites/accept?token=${encodeURIComponent(token)}`;
}

export class InviteState {
	open = $state(false);
	draft = $state<InviteDraft>(emptyDraft());
	sending = $state(false);
	/** Why the last send did not issue an invite — shown, never swallowed. */
	error = $state<string | null>(null);
	issued = $state<IssuedInvite | null>(null);

	constructor(
		private readonly create: (input: InviteInput) => Promise<IssuedInviteWire>,
		private readonly origin: string
	) {}

	start(): void {
		this.draft = emptyDraft();
		this.error = null;
		this.issued = null;
		this.open = true;
	}

	cancel(): void {
		this.open = false;
	}

	async submit(): Promise<void> {
		this.error = draftProblem(this.draft);
		if (this.error) return;
		const input = toInviteInput(this.draft);
		this.sending = true;
		try {
			const res = await this.create(input);
			this.issued = { email: res.email, access: input.access, link: inviteLink(this.origin, res.token) };
		} catch (e) {
			this.error = e instanceof DojoApiError ? e.message : 'the invite could not be issued';
		} finally {
			this.sending = false;
		}
	}
}
