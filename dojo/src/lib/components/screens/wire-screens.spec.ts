import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrEngagements from './ScrEngagements.svelte';
import ScrMyDojos from './ScrMyDojos.svelte';
import type { Engagement } from '$lib/client-data';
import type { DojoOrg } from '$lib/dojo-data';
import { toKitEngagements } from '$lib/client-map';
import { toKitDojos } from '$lib/chrome';
import { confidentialityFor } from '$lib/components/kit/fixtures';

// The Tier-1 dojo screens wired to /v1: this drives each screen through the SAME
// wire→kit mappers the loaders use, off MOCKED wire rows (Engagement / DojoOrg) —
// proving the whole wire → kit → screen path renders real-shaped data, and that
// each screen degrades to an honest-empty state when the client returns nothing
// (the DJ1 solo / dev-404 path). The confidentiality panel stays on the fixture
// (its route isn't built); ScrMyDojos reuses the layout memberships mapper
// (toKitDojos), not a new /v1 call.

const NOW = new Date('2026-07-23T12:00:00Z');

const wireEngagement: Engagement = {
	id: 'e1',
	client_name: 'Globex',
	client_tenant_id: null,
	description: null,
	project_bindings: [{ project_id: 'p1', name: 'globex-portal' }],
	policy_overrides: {},
	status: 'active',
	starts_on: '2026-07-16',
	ends_on: null,
	created_at: '2026-01-01T00:00:00Z',
	updated_at: '2026-07-01T00:00:00Z',
	lessons_kept: 0,
	stripped: 0
};

const wireOrg: DojoOrg = {
	id: 'acme',
	kanji: '社',
	name: 'Acme Corp',
	kind: 'Organization',
	host: 'saas',
	url: 'github/acme',
	role: 'Admin',
	from: 'member · admin',
	members: 48,
	pending: 4
};

describe('ScrEngagements — wired engagements', () => {
	afterEach(cleanup);

	it('renders an engagement row from a mapped wire engagement', () => {
		const engagements = toKitEngagements([wireEngagement], NOW);
		const { getByText } = render(ScrEngagements, {
			props: { orgName: 'Acme Corp', engagements, confidentiality: confidentialityFor('acme') }
		});
		expect(getByText('Globex')).toBeTruthy();
		expect(getByText(/globex-portal/)).toBeTruthy();
		// the confidentiality panel (still fixture-backed) renders alongside.
		expect(getByText('What crosses the boundary')).toBeTruthy();
	});

	it('renders the real per-engagement kept/stripped counts (was hardcoded 0)', () => {
		const engagements = toKitEngagements([{ ...wireEngagement, lessons_kept: 86, stripped: 214 }], NOW);
		const { getByText } = render(ScrEngagements, {
			props: { orgName: 'Acme Corp', engagements, confidentiality: confidentialityFor('acme') }
		});
		expect(getByText('86')).toBeTruthy(); // lessons kept (published)
		expect(getByText('214')).toBeTruthy(); // stripped (archived)
	});

	it('matches the rendered engagements register snapshot', () => {
		const engagements = toKitEngagements([{ ...wireEngagement, lessons_kept: 86, stripped: 214 }], NOW);
		const { container } = render(ScrEngagements, {
			props: { orgName: 'Acme Corp', engagements, confidentiality: confidentialityFor('acme') }
		});
		expect(container).toMatchSnapshot();
	});

	it('renders an empty register (count 0) when the client returns none', () => {
		const { getByText, queryByText } = render(ScrEngagements, {
			props: { orgName: 'Acme Corp', engagements: toKitEngagements([], NOW), confidentiality: confidentialityFor('acme') }
		});
		expect(getByText('Engagements')).toBeTruthy();
		expect(queryByText('Globex')).toBeNull();
	});
});

describe('ScrMyDojos — wired memberships', () => {
	afterEach(cleanup);

	it('renders a membership row from the mapped layout memberships', () => {
		const dojos = toKitDojos([wireOrg]);
		const { getByText } = render(ScrMyDojos, { props: { dojos } });
		expect(getByText('Acme Corp')).toBeTruthy();
	});

	it('shows the honest-empty state with no memberships (solo user, DJ1)', () => {
		const { getByText } = render(ScrMyDojos, { props: { dojos: toKitDojos([]) } });
		expect(getByText('No memberships yet — create or join a Dōjō.')).toBeTruthy();
	});
});
