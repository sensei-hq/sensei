import { render, cleanup, fireEvent, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoLibrary from './DojoLibrary.svelte';
import { createLibraryStore } from '$lib/library-state.svelte';

// Render tests for the constitution-library browser. Proves the screen renders
// the six areas, the packs of the selected area (with sources + recommended /
// regulated chips + checkers), and that the sticky footer count updates live as
// rules are toggled. Presentational — driven by the injected store; no fetch.

describe('DojoLibrary', () => {
	afterEach(() => cleanup());

	it('renders the header and all six areas in the rail', () => {
		const { getByText, getAllByRole, getByRole } = render(DojoLibrary, {
			store: createLibraryStore()
		});
		expect(getByText('Constitution library')).toBeTruthy();
		const tabs = getAllByRole('tab');
		expect(tabs.length).toBe(6);
		// Each area is a tab in the rail (the selected area's name also appears in
		// the main-column header, so scope the assertion to the tab role).
		expect(getByRole('tab', { name: /Core principles/ })).toBeTruthy();
		expect(getByRole('tab', { name: /Compliance/ })).toBeTruthy();
		expect(getByRole('tab', { name: /Design system & UI/ })).toBeTruthy();
	});

	it('renders the packs of the default (core) area with source + recommended chip', () => {
		const { getByText, getAllByText } = render(DojoLibrary, { store: createLibraryStore() });
		expect(getByText('First principles')).toBeTruthy();
		expect(getByText('SOLID')).toBeTruthy();
		expect(getByText('R. Martin')).toBeTruthy();
		// Two core packs are recommended (First principles, SOLID).
		expect(getAllByText('recommended').length).toBeGreaterThanOrEqual(2);
	});

	it('starts the footer at zero and updates the count when a rule is toggled', async () => {
		const { getByText, getByRole } = render(DojoLibrary, { store: createLibraryStore() });
		// Footer headline count starts at 0 rules selected.
		const footer = getByText('rules selected').closest('div')!;
		expect(within(footer).getByText('0')).toBeTruthy();
		// Toggle a rule (First principles' YAGNI) and the count moves to 1.
		await fireEvent.click(getByRole('button', { name: /YAGNI/ }));
		expect(within(footer).getByText('1')).toBeTruthy();
	});

	it('switches areas and shows the compliance packs + regulated chips', async () => {
		const { getByRole, getByText, getAllByText } = render(DojoLibrary, {
			store: createLibraryStore()
		});
		await fireEvent.click(getByRole('tab', { name: /Compliance/ }));
		expect(getByText('HIPAA')).toBeTruthy();
		expect(getByText('GDPR')).toBeTruthy();
		// Every compliance pack is regulated → the "法 regulated" chip appears.
		expect(getAllByText('法 regulated').length).toBeGreaterThanOrEqual(3);
	});

	it('surfaces the stack reviewer explainer + checker chips on the stack area', async () => {
		const { getByRole, getByText, getAllByText } = render(DojoLibrary, {
			store: createLibraryStore()
		});
		await fireEvent.click(getByRole('tab', { name: /Language & stack/ }));
		expect(getByText(/caught mechanically/)).toBeTruthy();
		// eslint appears as a wired checker on the TypeScript pack.
		expect(getAllByText('eslint').length).toBeGreaterThanOrEqual(1);
	});

	it('adds a written-your-own rule to the count', async () => {
		const store = createLibraryStore();
		const { getByLabelText, getByRole, getByText } = render(DojoLibrary, { store });
		const input = getByLabelText('New rule text');
		await fireEvent.input(input, { target: { value: 'Feature flags removed within two releases' } });
		await fireEvent.click(getByRole('button', { name: 'Add rule' }));
		const footer = getByText('rules selected').closest('div')!;
		expect(within(footer).getByText('1')).toBeTruthy();
		expect(store.selectedCount).toBe(1);
	});

	it('names the target scope in the footer', () => {
		const { getByText } = render(DojoLibrary, {
			store: createLibraryStore(),
			scopeName: 'Acme Corp'
		});
		expect(getByText('→ Acme Corp')).toBeTruthy();
	});
});
