import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import LegalPage from './LegalPage.svelte';
import { PRIVACY, TERMS } from '$lib/legal-content';

describe('LegalPage', () => {
	afterEach(cleanup);

	it('renders the title, the last-updated date and every section heading', () => {
		const { getByRole, container } = render(LegalPage, { page: PRIVACY });
		expect(getByRole('heading', { level: 1, name: PRIVACY.title })).toBeTruthy();
		expect(container.textContent).toContain(`Last updated ${PRIVACY.updated}`);
		for (const s of PRIVACY.sections) {
			expect(getByRole('heading', { level: 2, name: s.heading })).toBeTruthy();
		}
	});

	it('links to the other legal page and back to sign in', () => {
		const { getByRole } = render(LegalPage, { page: TERMS });
		expect(getByRole('link', { name: 'Privacy Policy' }).getAttribute('href')).toBe('/privacy-policy');
		expect(getByRole('link', { name: 'Sign in' }).getAttribute('href')).toBe('/signin');
	});
});
