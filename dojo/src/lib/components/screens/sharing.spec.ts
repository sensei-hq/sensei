// ScrSharing — the decision surface for `dojo.all_my_repositories`.
//
// The screen is presentational: it renders the verdict it is handed. What these
// pin is that it renders the RIGHT things — because the failure mode this whole
// slice exists to remove is a refusal that looks like "nothing to sync".
import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import ScrSharing from './ScrSharing.svelte';
import type { MyRepoWire } from '$lib/client-data';

function repo(over: Partial<MyRepoWire> = {}): MyRepoWire {
	return {
		repository_id: 'r1',
		repo_key: 'github.com/acme/api',
		name: 'api',
		tenant: 'organization/acme',
		owning_org: 'acme',
		forge_visibility: 'public',
		authority: 'user',
		may_share: true,
		elected: false,
		sync_enabled: false,
		configurable_by_me: true,
		reason_code: 'not_elected_user',
		reason: 'You have not turned sharing on for this repository',
		remedy: 'Turn sharing on for this repository',
		reason_actor: 'user',
		last_synced_at: null,
		metric_rows: 0,
		...over
	};
}

describe('ScrSharing — a refusal names itself', () => {
	it('shows the reason AND the remedy, not just "not sharing"', () => {
		const { getByText } = render(ScrSharing, { props: { repos: [repo()] } });
		expect(getByText(/You have not turned sharing on/)).toBeTruthy();
		expect(getByText(/Turn sharing on for this repository/)).toBeTruthy();
	});

	it('names the ACTOR when the caller cannot act', () => {
		// "Your org turned this off" vs "nobody has looked" — the distinction the
		// reason registry exists to carry.
		const { getByText } = render(ScrSharing, {
			props: {
				repos: [
					repo({
						authority: 'organization',
						configurable_by_me: false,
						reason_code: 'not_subscribed',
						reason: 'No active subscription for this organisation',
						remedy: 'Ask an admin to subscribe',
						reason_actor: 'admin'
					})
				]
			}
		});
		expect(getByText(/admin can change it/)).toBeTruthy();
	});

	it('says WHO holds the choice rather than assuming the user does', () => {
		const { getByText } = render(ScrSharing, {
			props: { repos: [repo({ authority: 'organization', owning_org: 'acme' })] }
		});
		expect(getByText(/acme decides/)).toBeTruthy();
	});

	it('reports an uncaptured forge answer as such, never as public', () => {
		// Rendering "public" for an unknown would hand the choice to the user when
		// it may be the organisation's — the exact failure the null state exists
		// to prevent.
		const { getByText } = render(ScrSharing, {
			props: { repos: [repo({ forge_visibility: null, authority: null })] }
		});
		expect(getByText(/not yet captured/)).toBeTruthy();
		expect(getByText(/nobody yet/)).toBeTruthy();
	});
});

describe('ScrSharing — the toggle', () => {
	it('reports the repo key and the NEW value upward', async () => {
		const onToggle = vi.fn();
		const { getByLabelText } = render(ScrSharing, { props: { repos: [repo()], onToggle } });
		await fireEvent.click(getByLabelText('Share api'));
		expect(onToggle).toHaveBeenCalledWith('github.com/acme/api', true);
	});

	it('is DISABLED when the caller may not configure it', async () => {
		// Otherwise the user clicks, the server 403s, and the row reverts — a
		// refusal discovered by trying rather than by being told.
		const onToggle = vi.fn();
		const { getByLabelText } = render(ScrSharing, {
			props: { repos: [repo({ configurable_by_me: false })], onToggle }
		});
		expect((getByLabelText('Share api') as HTMLInputElement).disabled).toBe(true);
	});

	it('is disabled while a toggle for that repo is in flight', () => {
		const { getByLabelText } = render(ScrSharing, {
			props: { repos: [repo()], pending: new Set(['github.com/acme/api']) }
		});
		expect((getByLabelText('Share api') as HTMLInputElement).disabled).toBe(true);
	});

	it('shows ELECTED and SYNCING separately — electing is only half the decision', () => {
		// A repo the user elected but that entitlement still refuses must not read
		// as "sharing", or "I turned it on and nothing happened" becomes a mystery.
		const { getByLabelText, getByText } = render(ScrSharing, {
			props: { repos: [repo({ elected: true, sync_enabled: false, may_share: false })] }
		});
		expect((getByLabelText('Share api') as HTMLInputElement).checked).toBe(true);
		expect(getByText('not sharing')).toBeTruthy();
	});
});

describe('ScrSharing — empty is not the same as broken', () => {
	it('renders an ERROR state, never an empty list, when the read failed', () => {
		const { getByRole, queryByText } = render(ScrSharing, {
			props: { repos: [], error: 'connection reset' }
		});
		expect(getByRole('alert').textContent).toMatch(/connection reset/);
		expect(queryByText(/No repositories yet/)).toBeNull();
	});

	it('renders the empty state only when there is genuinely nothing', () => {
		const { getByText } = render(ScrSharing, { props: { repos: [] } });
		expect(getByText(/No repositories yet/)).toBeTruthy();
	});
});
