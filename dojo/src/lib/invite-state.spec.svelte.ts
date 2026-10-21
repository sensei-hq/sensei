import { describe, expect, it, vi } from 'vitest';
import { InviteState, draftProblem, emptyDraft, inviteLink, toInviteInput } from '$lib/invite-state.svelte';
import { DojoApiError } from '$lib/client-data';

// The admin invite (#307 A3, D8): a member invite makes a membership at a role;
// a viewer invite makes a dōjō viewer — no role, no repositories — whom the admin
// then grants selected projects. The issued link is what the admin shares.

describe('invite draft rules', () => {
	it('a member invite carries its role', () => {
		const d = { ...emptyDraft(), email: ' Ada@Example.com ', role: 'maintainer' };
		expect(toInviteInput(d)).toEqual({ email: 'ada@example.com', kind: 'employer', access: 'member', role: 'maintainer' });
	});

	it('a viewer invite carries no role', () => {
		const d = { ...emptyDraft(), email: 'boss@example.com', access: 'viewer' as const, role: 'admin', kind: 'client' as const };
		expect(toInviteInput(d)).toEqual({ email: 'boss@example.com', kind: 'client', access: 'viewer' });
	});

	it('names the problem with an email that is not one', () => {
		expect(draftProblem({ ...emptyDraft(), email: '' })).toBe('Enter the email address to invite.');
		expect(draftProblem({ ...emptyDraft(), email: 'no-at-sign' })).toBe('That is not an email address.');
		expect(draftProblem({ ...emptyDraft(), email: 'a@b.co' })).toBeNull();
	});

	it('the link is the accept page with the token', () => {
		expect(inviteLink('https://dojo.example', 'tok/+=')).toBe('https://dojo.example/you/invites/accept?token=tok%2F%2B%3D');
	});
});

describe('InviteState', () => {
	const issued = { id: 'i1', token: 'tk', email: 'boss@example.com', role: 'contributor', expires_at: '2026-10-28T00:00:00Z' };

	it('opens with an empty draft and closes', () => {
		const s = new InviteState(vi.fn(), 'https://d');
		expect(s.open).toBe(false);
		s.start();
		expect(s.open).toBe(true);
		expect(s.draft).toEqual(emptyDraft());
		s.cancel();
		expect(s.open).toBe(false);
	});

	it('starting from the Viewers tab presets a viewer invite', () => {
		const s = new InviteState(vi.fn(), 'https://d');
		s.start('viewer');
		expect(s.draft.access).toBe('viewer');
	});

	it('issues the invite and shows the link to share', async () => {
		const create = vi.fn().mockResolvedValue(issued);
		const s = new InviteState(create, 'https://d');
		s.start();
		s.draft = { ...s.draft, email: 'boss@example.com', access: 'viewer', kind: 'client' };
		await s.submit();
		expect(create).toHaveBeenCalledWith({ email: 'boss@example.com', kind: 'client', access: 'viewer' });
		expect(s.issued).toEqual({ email: 'boss@example.com', access: 'viewer', link: 'https://d/you/invites/accept?token=tk' });
		expect(s.error).toBeNull();
		expect(s.sending).toBe(false);
	});

	it('does not send a draft with a problem, and says what it is', async () => {
		const create = vi.fn();
		const s = new InviteState(create, 'https://d');
		s.start();
		await s.submit();
		expect(create).not.toHaveBeenCalled();
		expect(s.error).toBe('Enter the email address to invite.');
	});

	it('says why the dōjō refused, and issues nothing', async () => {
		const s = new InviteState(vi.fn().mockRejectedValue(new DojoApiError(403, 'admins only')), 'https://d');
		s.start();
		s.draft = { ...s.draft, email: 'a@b.co' };
		await s.submit();
		expect(s.error).toBe('admins only');
		expect(s.issued).toBeNull();
		expect(s.sending).toBe(false);
	});

	it('starting again clears the last link and error', async () => {
		const s = new InviteState(vi.fn().mockResolvedValue(issued), 'https://d');
		s.start();
		s.draft = { ...s.draft, email: 'a@b.co' };
		await s.submit();
		s.start();
		expect(s.issued).toBeNull();
		expect(s.error).toBeNull();
	});
});
