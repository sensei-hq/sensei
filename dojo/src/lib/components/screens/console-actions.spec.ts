import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import ScrIncidents from './ScrIncidents.svelte';
import ScrEngagements from './ScrEngagements.svelte';
import type { KitMember, KitIncident, KitEngagement, KitConfidentiality } from '$lib/components/kit/types';

// The Tier-3 console ACTION affordances: this drives each wired screen's
// mutation controls (set-role select, incident report/resolve/delete, engagement
// new/close/delete) and proves each calls its callback with the right payload —
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

const incident = (over: Partial<KitIncident> = {}): KitIncident => ({
	id: 'i1',
	kanji: '盾',
	title: 'Near-leak',
	client: 'acme',
	state: 'open',
	when: '2d',
	severity: 'high',
	...over
});

const engagement = (over: Partial<KitEngagement> = {}): KitEngagement => ({
	id: 'e1',
	kanji: '客',
	client: 'Acme',
	clientTenantId: null,
	projects: '—',
	lessons: 0,
	dropped: 0,
	since: '3mo',
	status: 'active',
	...over
});

const CONF: KitConfidentiality = { kept: [], dropped: [], example: { raw: 'a', stripped: 'b' } };

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

describe('ScrIncidents — report / resolve / delete affordances', () => {
	afterEach(cleanup);

	it('Report prompts for a title and calls onReport with it', async () => {
		const onReport = vi.fn();
		const spy = vi.spyOn(window, 'prompt').mockReturnValue('New incident');
		const { getByText } = render(ScrIncidents, { props: { orgName: 'Acme', incidents: [], onReport } });
		await fireEvent.click(getByText('Report'));
		expect(onReport).toHaveBeenCalledWith('New incident');
		spy.mockRestore();
	});

	it('Report is a no-op when the prompt is cancelled', async () => {
		const onReport = vi.fn();
		const spy = vi.spyOn(window, 'prompt').mockReturnValue(null);
		const { getByText } = render(ScrIncidents, { props: { orgName: 'Acme', incidents: [], onReport } });
		await fireEvent.click(getByText('Report'));
		expect(onReport).not.toHaveBeenCalled();
		spy.mockRestore();
	});

	it('Resolve calls onResolve; hidden once resolved', async () => {
		const onResolve = vi.fn();
		const { getByText, queryByText, rerender } = render(ScrIncidents, {
			props: { orgName: 'Acme', incidents: [incident({ state: 'open' })], onResolve }
		});
		await fireEvent.click(getByText('Resolve'));
		expect(onResolve.mock.calls[0][0].id).toBe('i1');
		await rerender({ orgName: 'Acme', incidents: [incident({ state: 'resolved' })], onResolve });
		expect(queryByText('Resolve')).toBeNull();
	});

	it('Delete calls onDelete', async () => {
		const onDelete = vi.fn();
		const { getByText } = render(ScrIncidents, {
			props: { orgName: 'Acme', incidents: [incident()], onDelete }
		});
		await fireEvent.click(getByText('Delete'));
		expect(onDelete.mock.calls[0][0].id).toBe('i1');
	});
});

describe('ScrEngagements — new / close / delete affordances', () => {
	afterEach(cleanup);

	it('New engagement prompts for a client and calls onNew', async () => {
		const onNew = vi.fn();
		const spy = vi.spyOn(window, 'prompt').mockReturnValue('Globex');
		const { getByText } = render(ScrEngagements, {
			props: { orgName: 'Acme', engagements: [], confidentiality: CONF, onNew }
		});
		await fireEvent.click(getByText('New engagement'));
		expect(onNew).toHaveBeenCalledWith('Globex');
		spy.mockRestore();
	});

	it('Close calls onClose; hidden once ended', async () => {
		const onClose = vi.fn();
		const { getByText, queryByText, rerender } = render(ScrEngagements, {
			props: { orgName: 'Acme', engagements: [engagement({ status: 'active' })], confidentiality: CONF, onClose }
		});
		await fireEvent.click(getByText('Close'));
		expect(onClose.mock.calls[0][0].id).toBe('e1');
		await rerender({ orgName: 'Acme', engagements: [engagement({ status: 'ended' })], confidentiality: CONF, onClose });
		expect(queryByText('Close')).toBeNull();
	});

	it('Delete calls onDelete', async () => {
		const onDelete = vi.fn();
		const { getByText } = render(ScrEngagements, {
			props: { orgName: 'Acme', engagements: [engagement()], confidentiality: CONF, onDelete }
		});
		await fireEvent.click(getByText('Delete'));
		expect(onDelete.mock.calls[0][0].id).toBe('e1');
	});

	it('stays read-only (no Close/Delete) when the callbacks are absent', () => {
		const { queryByText } = render(ScrEngagements, {
			props: { orgName: 'Acme', engagements: [engagement()], confidentiality: CONF }
		});
		expect(queryByText('Close')).toBeNull();
		expect(queryByText('Delete')).toBeNull();
	});
});
