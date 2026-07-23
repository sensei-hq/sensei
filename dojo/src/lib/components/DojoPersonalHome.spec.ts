import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoPersonalHome from './DojoPersonalHome.svelte';

// DojoPersonalHome (DJ1 solo landing) render tests. The screen a signed-in user
// with NO Dōjō membership sees. Presentational — takes `user`, no fetch. Proves
// the honest-empty blocks (no fabricated project rows), the library link, and
// the clearly-secondary create/join affordance.

describe('DojoPersonalHome', () => {
	afterEach(() => cleanup());

	it('greets the solo user by their display name', () => {
		const { getByText } = render(DojoPersonalHome, { user: { name: 'Rin Saito', email: 'rin@x.dev' } });
		// The eyebrow greeting names the signed-in user and marks them as solo.
		expect(getByText(/signed in as Rin Saito · working solo/)).toBeTruthy();
	});

	it('renders honest-empty projects — no fabricated rows, points at the local desktop app', () => {
		const { getByText } = render(DojoPersonalHome, { user: { email: 'rin@x.dev' } });
		expect(getByText(/live on your own machine/)).toBeTruthy();
		expect(getByText(/local · this machine/)).toBeTruthy();
	});

	it('renders an honest-empty "needs you" placeholder', () => {
		const { getByText } = render(DojoPersonalHome, { user: {} });
		expect(getByText(/needs you/i)).toBeTruthy();
		expect(getByText(/nothing here is waiting on you/)).toBeTruthy();
	});

	it('offers a "your own rules · optional" card linking to the library', () => {
		const { getByText, container } = render(DojoPersonalHome, { user: {} });
		expect(getByText(/your own rules · optional/)).toBeTruthy();
		const libraryLink = container.querySelector('a[href="/console/library"]');
		expect(libraryLink).toBeTruthy();
		expect(libraryLink?.textContent).toMatch(/open library/);
	});

	it('offers a clearly-optional create/join Dōjō block routed to the org picker', () => {
		const { getByText, container } = render(DojoPersonalHome, { user: {} });
		expect(getByText(/create or join a Dōjō · optional/)).toBeTruthy();
		// Both create + join link to /orgs (resolve stub returns the route id).
		const orgLinks = [...container.querySelectorAll('a[href="/orgs"]')];
		expect(orgLinks.length).toBe(2);
	});

	it('never crashes without a user (SSR-safe / magic-link with no profile)', () => {
		const { getByText } = render(DojoPersonalHome, {});
		expect(getByText(/signed in as you · working solo/)).toBeTruthy();
	});
});
