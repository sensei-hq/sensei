// Pure helpers for the developer / personal console (Chunk 4). All the counts,
// tallies and per-membership overlay the three sections need — the pending
// contributions badge, the unread ("new") downstream count, the per-destination
// status tallies, and the role / "follows" for a membership — live here as
// side-effect-free functions over the `developer-data` catalog and the caller's
// real memberships, so they unit-test without a DOM and the components stay thin.

import {
	CONTRIBUTIONS,
	DEV_FOLLOWS,
	DEV_ROLE_BY_KIND,
	DOWNSTREAM,
	type Contribution,
	type ContributionStatus,
	type Downstream
} from './developer-data';
import type { DojoOrg, OrgKind } from './dojo-data';

// ── My teams: per-membership overlay ─────────────────────────────────────────

/** The role a contributor holds in a membership of this kind (mockup
 *  `DEV_ROLE_BY_KIND`), falling back to a neutral "Member" for an unknown kind. */
export function roleForKind(kind: OrgKind): string {
	return DEV_ROLE_BY_KIND[kind] ?? 'Member';
}

/** What a membership follows (mockup `DEV_FOLLOWS`, keyed on id), falling back to
 *  a neutral label for a membership without an authored entry (a real membership
 *  the sample overlay doesn't cover yet). */
export function followsForMembership(id: string): string {
	return DEV_FOLLOWS[id] ?? 'your projects';
}

// ── My contributions: status tallies ─────────────────────────────────────────

/** Every contribution the contributor has sent upstream (newest-intent first). */
export function allContributions(): readonly Contribution[] {
	return CONTRIBUTIONS;
}

/** How many contributions are in a given status. */
export function contributionsByStatus(status: ContributionStatus): number {
	return CONTRIBUTIONS.filter((c) => c.status === status).length;
}

/** The count that drives the "My contributions" nav badge + the header summary:
 *  how many sends are still awaiting a maintainer's decision (pending). */
export function pendingContributionCount(): number {
	return contributionsByStatus('pending');
}

/** The full status tally the header renders ("2 approved · 1 pending"). */
export interface ContributionTally {
	approved: number;
	pending: number;
	declined: number;
	total: number;
}

/** The approved / pending / declined tally across all contributions. */
export function contributionTally(): ContributionTally {
	return {
		approved: contributionsByStatus('approved'),
		pending: contributionsByStatus('pending'),
		declined: contributionsByStatus('declined'),
		total: CONTRIBUTIONS.length
	};
}

/** Metadata for a status chip — the tone token class + the human label. */
export interface StatusMeta {
	toneClass: string;
	label: string;
}

/** The chip tone + label for a contribution status (mockup `statusMeta`). */
export function statusMeta(status: ContributionStatus): StatusMeta {
	switch (status) {
		case 'approved':
			return { toneClass: 'text-success', label: 'approved' };
		case 'pending':
			return { toneClass: 'text-accent', label: 'in triage' };
		case 'declined':
			return { toneClass: 'text-danger', label: 'declined' };
	}
}

// ── For me: downstream tallies ───────────────────────────────────────────────

/** Every teaching distributed down to the contributor (newest first). */
export function allDownstream(): readonly Downstream[] {
	return DOWNSTREAM;
}

/** How many distributed teachings are still new (not yet adopted) — the "For me"
 *  unread count the header + a future nav badge read. */
export function unreadDownstreamCount(): number {
	return DOWNSTREAM.filter((d) => !d.adopted).length;
}

/** How many distributed teachings the contributor has already adopted. */
export function adoptedDownstreamCount(): number {
	return DOWNSTREAM.filter((d) => d.adopted).length;
}

/** How many distinct memberships the downstream teachings arrived from (the
 *  header's "across N memberships"). */
export function downstreamSourceCount(): number {
	return new Set(DOWNSTREAM.map((d) => d.from)).size;
}
