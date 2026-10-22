import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrMyDojos from './ScrMyDojos.svelte';
import type { DojoOrg } from '$lib/dojo-data';
import { toKitDojos } from '$lib/chrome';

// The Tier-1 dojo screens wired to /v1: this drives each screen through the SAME
// wire→kit mappers the loaders use, off MOCKED wire rows (DojoOrg) — proving the
// whole wire → kit → screen path renders real-shaped data, and that each screen
// degrades to an honest-empty state when the client returns nothing (the DJ1
// solo / dev-404 path). ScrMyDojos reuses the layout memberships mapper
// (toKitDojos), not a new /v1 call.

const wireOrg: DojoOrg = {
	id: 'acme',
	kanji: '社',
	name: 'Acme Corp',
	kind: 'Organization',
	url: 'github/acme',
	role: 'Admin',
	from: 'member · admin',
	members: 48,
	pending: 4
};

describe('ScrMyDojos — wired memberships', () => {
	afterEach(cleanup);

	it('renders a membership row from the mapped layout memberships', () => {
		const dojos = toKitDojos([wireOrg]);
		const { getByText } = render(ScrMyDojos, { props: { dojos } });
		expect(getByText('Acme Corp')).toBeTruthy();
	});

	it('shows the honest-empty state with no memberships (solo user, DJ1)', () => {
		const { getByText } = render(ScrMyDojos, { props: { dojos: toKitDojos([]) } });
		expect(getByText('No dōjōs yet.')).toBeTruthy();
	});
});
