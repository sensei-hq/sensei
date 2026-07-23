import { describe, expect, it } from 'vitest';
import { ROLE_SURFACE_TABS, resolveTab, tabForSection } from './dojo2-role-surfaces-view';

// The admin role-surfaces helpers — the three-tab config + the nav-id → tab
// mapping that lets one screen serve both the `members` and `audit` nav ids.
// Pure functions, no DOM.

describe('ROLE_SURFACE_TABS', () => {
	it('is Members & Roles · Policies · Audit in nav order', () => {
		expect(ROLE_SURFACE_TABS.map((t) => t.id)).toEqual(['members', 'policies', 'audit']);
	});

	it('drives a distinct header + CTA per tab', () => {
		const [members, policies, audit] = ROLE_SURFACE_TABS;
		expect(members.title).toBe('Members & Roles');
		expect(members.cta).toBe('Invite');
		expect(policies.title).toBe('Role policies');
		expect(policies.cta).toBe('New policy');
		expect(audit.title).toBe('Audit log');
		expect(audit.cta).toBe('Export');
	});
});

describe('resolveTab', () => {
	it('resolves a known tab id', () => {
		expect(resolveTab('policies').id).toBe('policies');
		expect(resolveTab('audit').title).toBe('Audit log');
	});

	it('falls back to Members for an unknown id', () => {
		expect(resolveTab('bogus').id).toBe('members');
	});
});

describe('tabForSection — one surface, two nav ids', () => {
	it('opens the Members tab for the members section', () => {
		expect(tabForSection('members')).toBe('members');
	});

	it('opens the Audit tab for the audit section', () => {
		expect(tabForSection('audit')).toBe('audit');
	});

	it('defaults any other section to Members', () => {
		expect(tabForSection('anything')).toBe('members');
	});
});
