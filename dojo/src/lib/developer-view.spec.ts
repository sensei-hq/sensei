import { describe, expect, it } from 'vitest';
import {
	adoptedDownstreamCount,
	allContributions,
	allDownstream,
	clientMembershipCount,
	contributionsByStatus,
	contributionTally,
	downstreamSourceCount,
	followsForMembership,
	isAnonymizedMembership,
	pendingContributionCount,
	roleForKind,
	statusMeta,
	unreadDownstreamCount
} from '$lib/developer-view';
import { CONTRIBUTIONS, DOWNSTREAM } from '$lib/developer-data';
import type { DojoOrg, OrgKind } from '$lib/dojo-data';

// Pure count/tally/overlay tests for the developer console. Exercises the four
// things the three sections depend on: the per-membership role + "follows"
// overlay (My teams), the per-status contribution tally + pending badge (My
// contributions), and the unread/adopted downstream counts (For me).

// A minimal membership fixture — id + kind is all the overlay reads.
function membership(id: string, kind: OrgKind): DojoOrg {
	return {
		id,
		kanji: '社',
		name: id,
		kind,
		host: 'saas',
		url: `github/${id}`,
		role: 'Contributor',
		from: 'test',
		members: 1,
		pending: 0
	};
}

describe('My teams overlay', () => {
	it('roleForKind maps each kind to the mockup role', () => {
		expect(roleForKind('Organization')).toBe('Contributor');
		expect(roleForKind('Client')).toBe('Contributor · anonymized');
		expect(roleForKind('Community')).toBe('Member');
		expect(roleForKind('Personal')).toBe('Owner');
	});

	it('roleForKind falls back to Member for an unknown kind', () => {
		// A kind outside the union (defensive; the type keeps callers honest).
		expect(roleForKind('Nonprofit' as OrgKind)).toBe('Member');
	});

	it('followsForMembership reads the authored overlay by id', () => {
		expect(followsForMembership('acme')).toBe('Web · Auth · Payments');
		expect(followsForMembership('rustco')).toBe('rust · axum · sqlx');
		expect(followsForMembership('self')).toBe('everything (private)');
	});

	it('followsForMembership falls back for a membership the overlay does not cover', () => {
		expect(followsForMembership('brand-new-org')).toBe('your projects');
	});

	it('isAnonymizedMembership is true only for client Dōjōs', () => {
		expect(isAnonymizedMembership('Client')).toBe(true);
		expect(isAnonymizedMembership('Organization')).toBe(false);
		expect(isAnonymizedMembership('Community')).toBe(false);
		expect(isAnonymizedMembership('Personal')).toBe(false);
	});

	it('clientMembershipCount counts only client memberships', () => {
		const memberships = [
			membership('acme', 'Organization'),
			membership('globex', 'Client'),
			membership('initech', 'Client'),
			membership('self', 'Personal')
		];
		expect(clientMembershipCount(memberships)).toBe(2);
		expect(clientMembershipCount([])).toBe(0);
	});
});

describe('My contributions tallies', () => {
	it('allContributions returns the catalog', () => {
		expect(allContributions()).toBe(CONTRIBUTIONS);
		expect(allContributions().length).toBe(4);
	});

	it('contributionsByStatus counts each status against the catalog', () => {
		expect(contributionsByStatus('approved')).toBe(2);
		expect(contributionsByStatus('pending')).toBe(1);
		expect(contributionsByStatus('declined')).toBe(1);
	});

	it('pendingContributionCount is the pending tally (the nav badge)', () => {
		expect(pendingContributionCount()).toBe(1);
	});

	it('contributionTally sums to the catalog size', () => {
		const t = contributionTally();
		expect(t).toEqual({ approved: 2, pending: 1, declined: 1, total: 4 });
		expect(t.approved + t.pending + t.declined).toBe(t.total);
	});

	it('statusMeta gives a token tone class + label per status', () => {
		expect(statusMeta('approved')).toEqual({ toneClass: 'text-success', label: 'approved' });
		expect(statusMeta('pending')).toEqual({ toneClass: 'text-accent', label: 'in triage' });
		expect(statusMeta('declined')).toEqual({ toneClass: 'text-danger', label: 'declined' });
	});
});

describe('For me (downstream) tallies', () => {
	it('allDownstream returns the catalog', () => {
		expect(allDownstream()).toBe(DOWNSTREAM);
		expect(allDownstream().length).toBe(3);
	});

	it('unreadDownstreamCount counts the not-yet-adopted teachings', () => {
		expect(unreadDownstreamCount()).toBe(2);
	});

	it('adoptedDownstreamCount counts the adopted teachings', () => {
		expect(adoptedDownstreamCount()).toBe(1);
	});

	it('unread + adopted equals the catalog size', () => {
		expect(unreadDownstreamCount() + adoptedDownstreamCount()).toBe(DOWNSTREAM.length);
	});

	it('downstreamSourceCount counts distinct originating Dōjōs', () => {
		// Acme Corp (×2) + Rust Guild → 2 distinct sources.
		expect(downstreamSourceCount()).toBe(2);
	});
});
