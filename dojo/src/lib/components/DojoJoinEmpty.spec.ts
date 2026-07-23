import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoJoinEmpty from './DojoJoinEmpty.svelte';

// DojoJoinEmpty (DJ1 shared empty state) render tests. Every org-scoped console
// screen renders this when the caller has no membership — a calm "join or
// create a Dōjō" prompt in place of a fabricated tenant's data. Presentational —
// takes the screen name (`what`) and links to the org picker.

describe('DojoJoinEmpty', () => {
	afterEach(() => cleanup());

	it('names the screen in the copy and links to the org picker', () => {
		const { getByText, container } = render(DojoJoinEmpty, { what: 'members' });
		expect(getByText(/members lives inside a Dōjō/)).toBeTruthy();
		const link = container.querySelector('a[href="/orgs"]');
		expect(link).toBeTruthy();
		expect(link?.textContent).toMatch(/join or create a Dōjō/);
	});

	it('reassures the solo user they can keep working', () => {
		const { getByText } = render(DojoJoinEmpty, { what: 'triage' });
		expect(getByText(/keep working solo/)).toBeTruthy();
	});

	it('falls back to a neutral noun when no screen name is given', () => {
		const { getByText } = render(DojoJoinEmpty, {});
		expect(getByText(/this lives inside a Dōjō/)).toBeTruthy();
	});
});
