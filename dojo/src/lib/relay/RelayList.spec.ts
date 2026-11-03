import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import RelayList from './RelayList.svelte';
import { relayInboxState } from './relay-inbox-state.svelte';
import { relayInboxMock } from './relay-inbox.mock';
import { RelayNotify } from './relay-notify.svelte';

// #104: the rail rendered a relay outage as "Nothing in flight." and showed nothing
// while it loaded. Each load outcome now has its own state on screen.
afterEach(() => {
	cleanup();
	relayInboxState.load([]);
});

const outcome = (over: Partial<Parameters<typeof relayInboxState.loaded>[0]>) => ({
	sessions: [],
	tenants: 2,
	unreachable: [],
	plansMissing: 0,
	...over
});

describe('RelayList', () => {
	it('shows a skeleton while loading, not the empty inbox', () => {
		relayInboxState.begin();
		const { container, queryByText } = render(RelayList);
		expect(container.querySelector('[data-inbox-loading]')).not.toBeNull();
		expect(queryByText('Nothing in flight.')).toBeNull();
	});

	it('shows an error with a retry when no dōjō could be reached', () => {
		relayInboxState.begin();
		relayInboxState.loaded(outcome({ unreachable: ['one', 'two'] }));
		const { container, getByText, queryByText } = render(RelayList);
		expect(container.querySelector('[data-inbox-error]')).not.toBeNull();
		expect(getByText('Try again')).toBeTruthy();
		expect(queryByText('Nothing in flight.')).toBeNull();
	});

	it('lists what loaded and names what did not', () => {
		relayInboxState.begin();
		relayInboxState.loaded(outcome({ sessions: relayInboxMock(), unreachable: ['two'], plansMissing: 1 }));
		relayInboxState.setFilter('all');
		const { container, getByText } = render(RelayList);
		const partial = container.querySelector('[data-inbox-partial]');
		expect(partial?.textContent).toContain('two');
		expect(getByText('1 run(s) are shown without their plan.')).toBeTruthy();
		expect(container.querySelector('[data-inbox-error]')).toBeNull();
	});

	it('an inbox that truly is empty says so, with no warning', () => {
		relayInboxState.begin();
		relayInboxState.loaded(outcome({}));
		const { container, getByText } = render(RelayList);
		expect(getByText('Nothing in flight.')).toBeTruthy();
		expect(container.querySelector('[data-inbox-partial]')).toBeNull();
	});

	// The user (2026-10-28): "Notify me" goes into the relay inbox header.
	it('shows "Notify me" in the header, for the dōjōs the inbox spans', async () => {
		const subscribe = async () => ({ ok: true as const, id: 's' });
		const notify = new RelayNotify({ supported: () => true, permission: () => 'default', subscribe, postPrefs: async () => {} });
		relayInboxState.loaded(outcome({}));
		const { container, getByRole } = render(RelayList, { notify, tenantKeys: ['github.com/acme'] });
		expect(container.querySelector('[data-relay-notify="button"]')).toBeTruthy();
		getByRole('button', { name: /Notify me/ }).click();
		await new Promise((r) => setTimeout(r, 0));
		expect(container.querySelector('[data-relay-notify="on"]')?.textContent).toContain('Notifications on');
	});
});
