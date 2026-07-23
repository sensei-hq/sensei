import { render, cleanup, fireEvent, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import DojoRulePreview from './DojoRulePreview.svelte';
import { createPreviewStore } from '$lib/preview-state.svelte';

// Render tests for the effective-constitution preview. Proves the screen renders
// the ladder rungs (broad → specific) with per-rule tags, the "conflicts,
// resolved" cards, and that selecting a project + reclassifying (company↔client)
// changes the resolved result. Presentational — driven by the injected store; no
// fetch.

describe('DojoRulePreview', () => {
	afterEach(() => cleanup());

	it('renders the header and the four sample projects in the picker', () => {
		const { getByText, getAllByRole, getByRole } = render(DojoRulePreview, {
			store: createPreviewStore('globex')
		});
		expect(getByText('What governs this project')).toBeTruthy();
		const tabs = getAllByRole('tab');
		expect(tabs.length).toBe(4);
		// Each sample project is a picker tab (its name also appears as a ladder rung
		// label, so scope the assertion to the tab role).
		expect(getByRole('tab', { name: /lumen-auth/ })).toBeTruthy();
		expect(getByRole('tab', { name: /globex-portal/ })).toBeTruthy();
		expect(getByRole('tab', { name: /personal-site/ })).toBeTruthy();
		expect(getByRole('tab', { name: /agency-monorepo/ })).toBeTruthy();
	});

	it('renders the ladder rungs for a client project (Company + Client both apply)', () => {
		const { getByText, getAllByText } = render(DojoRulePreview, {
			store: createPreviewStore('globex')
		});
		expect(getByText('The ladder')).toBeTruthy();
		// "Company"/"Client" appear both as a ladder rung name and as a classification
		// chip/button, so assert presence via getAllByText.
		expect(getAllByText('Company').length).toBeGreaterThanOrEqual(1);
		expect(getAllByText('Client').length).toBeGreaterThanOrEqual(1);
		expect(getByText('Personal')).toBeTruthy();
		expect(getByText('Project')).toBeTruthy();
		expect(getByText('Stack')).toBeTruthy();
	});

	it('renders the conflicts-resolved cards with a locked coverage conflict', () => {
		const { getByText, getAllByText } = render(DojoRulePreview, {
			store: createPreviewStore('globex')
		});
		expect(getByText('Conflicts, resolved')).toBeTruthy();
		// The company coverage bar wins over the project relax, marked locked. The
		// winner text appears on both the ladder rung and the conflict card.
		expect(
			getAllByText('Test coverage ≥ 80% on money- or auth-touching paths').length
		).toBeGreaterThanOrEqual(1);
		expect(getAllByText(/wins/).length).toBeGreaterThanOrEqual(1);
		expect(getAllByText(/locked/).length).toBeGreaterThanOrEqual(1);
	});

	it('tags an overridden rule on the ladder (the project relax lost to the company lock)', () => {
		const { getAllByText } = render(DojoRulePreview, { store: createPreviewStore('globex') });
		// The "relax coverage to ≥ 60%" project rule is overridden by the company lock.
		expect(getAllByText(/overridden/).length).toBeGreaterThanOrEqual(1);
	});

	it('selecting the personal project drops the Company and Client rungs', async () => {
		const store = createPreviewStore('globex');
		const { getAllByText, queryByText, getByRole } = render(DojoRulePreview, { store });
		expect(getAllByText('Company').length).toBeGreaterThanOrEqual(1);
		await fireEvent.click(getByRole('tab', { name: /personal-site/ }));
		// A personal project has no company/client rung; the picker chips read
		// "社 company"/"客 client" (kanji-prefixed) so bare rung names are gone.
		expect(store.constitution.ladder.some((r) => r.scope === 'company')).toBe(false);
		expect(store.constitution.ladder.some((r) => r.scope === 'client')).toBe(false);
		expect(queryByText('Personal')).toBeTruthy();
	});

	it('reclassifying globex to company switches the Client rung off', async () => {
		const store = createPreviewStore('globex');
		const { getAllByText, getByRole } = render(DojoRulePreview, { store });
		expect(getAllByText('Client').length).toBeGreaterThanOrEqual(1);
		// Reclassify company↔client via the override buttons in the banner.
		await fireEvent.click(getByRole('button', { name: 'Company' }));
		expect(store.effectiveKind).toBe('company');
		expect(store.constitution.ladder.some((r) => r.scope === 'client')).toBe(false);
	});

	it('shows the summary card with the rule + non-negotiable counts', () => {
		const store = createPreviewStore('globex');
		const { getByText } = render(DojoRulePreview, { store });
		expect(getByText('rules govern this project on day one')).toBeTruthy();
		expect(getByText(`★ ${store.constitution.lockedCount} non-negotiable`)).toBeTruthy();
		const summary = getByText('rules govern this project on day one').closest('div')!;
		expect(within(summary).getByText(String(store.constitution.totalRules))).toBeTruthy();
	});

	it('the agency monorepo shows two client rungs (Globex + Initech)', () => {
		const { getByText } = render(DojoRulePreview, { store: createPreviewStore('mono') });
		expect(getByText('Globex')).toBeTruthy();
		expect(getByText('Initech')).toBeTruthy();
	});
});
