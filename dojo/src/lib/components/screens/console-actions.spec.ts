import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import type { KitMember } from '$lib/components/kit/types';

// The Tier-3 console ACTION affordances: this drives each wired screen's
// mutation controls (the set-role select) and proves each calls its callback with the right payload —
// the presentational → callback contract the org `+page.svelte` binds to the /v1
// write clients. When a callback is ABSENT the screen stays read-only (no
// affordance rendered), which the same tests assert.

const member = (over: Partial<KitMember> = {}): KitMember => ({
	name: 'alice',
	userId: 'u-alice',
	git: 'GitHub',
	role: 'contributor',
	scopes: '—',
	active: 'now',
	...over
});

describe('ScrRoleSurfaces — set-role affordance', () => {
	afterEach(cleanup);

	it('renders a role select and calls onSetRole with the chosen role', async () => {
		const onSetRole = vi.fn();
		const { getByLabelText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme', tab: 'members', members: [member()], onSetRole }
		});
		const sel = getByLabelText('Role for alice') as HTMLSelectElement;
		await fireEvent.change(sel, { target: { value: 'lead' } });
		expect(onSetRole).toHaveBeenCalledOnce();
		expect(onSetRole.mock.calls[0][0].userId).toBe('u-alice');
		expect(onSetRole.mock.calls[0][1]).toBe('lead');
	});

	it('stays read-only (no select) for the viewer own row', () => {
		const onSetRole = vi.fn();
		const { queryByLabelText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme', tab: 'members', members: [member({ name: 'me', you: true })], onSetRole }
		});
		expect(queryByLabelText('Role for me')).toBeNull();
	});

	it('stays read-only when onSetRole is absent (fixture render)', () => {
		const { queryByLabelText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme', tab: 'members', members: [member()] }
		});
		expect(queryByLabelText('Role for alice')).toBeNull();
	});
});
