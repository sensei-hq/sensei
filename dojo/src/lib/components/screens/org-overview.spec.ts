import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrOrgHome from './ScrOrgHome.svelte';
import ScrOrgLadder from './ScrOrgLadder.svelte';
import RuleEditor from '$lib/components/kit/RuleEditor.svelte';
import { orgWorkspacesFor, orgConstitutionFor, needsYou } from '$lib/components/kit/fixtures';

// Chunk-4 org Overview screens (the /org/[slug] zone core). Each renders off the
// ported org fixtures (presentational — real /v1 wiring is a later chunk). We
// assert the org home shows its jurisdiction workspaces + needs band + stat row,
// the ladder groups its sections + toggles a rule's include state + opens/saves/
// validates the RuleEditor, and the editor composes a rule from its draft.

const workspaces = orgWorkspacesFor('acme');
const sections = orgConstitutionFor('acme');
const needs = needsYou.slice(0, 2);
const stats = { members: 48, needs: 4, workspaces: 9 };

describe('ScrOrgHome — jurisdiction workspaces + needs + stats', () => {
	afterEach(cleanup);

	it('heads the jurisdiction and lists its workspaces', () => {
		const { getByText } = render(ScrOrgHome, {
			props: { orgName: 'Acme Corp', workspaces, needs, stats }
		});
		expect(getByText('Acme Corp · jurisdiction')).toBeTruthy();
		expect(getByText('4 workspaces under this dōjō')).toBeTruthy();
		// a jurisdiction workspace renders by name (org repo path).
		expect(getByText('lumen-auth')).toBeTruthy();
		expect(getByText('acme/lumen-auth')).toBeTruthy();
	});

	it('renders the org stat row (members · need a maintainer · workspaces)', () => {
		const { getByText } = render(ScrOrgHome, {
			props: { orgName: 'Acme Corp', workspaces, needs, stats }
		});
		expect(getByText('members')).toBeTruthy();
		expect(getByText('need a maintainer')).toBeTruthy();
		expect(getByText('48')).toBeTruthy();
	});

	it('renders the org needs band under the maintainer title', () => {
		const { getByText } = render(ScrOrgHome, {
			props: { orgName: 'Acme Corp', workspaces, needs, stats }
		});
		expect(getByText('Needs a maintainer')).toBeTruthy();
	});

	it('fires onOpenWorkspace when a workspace row is clicked', async () => {
		const onOpenWorkspace = vi.fn();
		const { getByText } = render(ScrOrgHome, {
			props: { orgName: 'Acme Corp', workspaces, needs, stats, onOpenWorkspace }
		});
		await fireEvent.click(getByText('lumen-auth'));
		expect(onOpenWorkspace).toHaveBeenCalledWith(workspaces.find((p) => p.name === 'lumen-auth'));
	});

	it('shows a failed queue read as an error in place of the band (#241)', () => {
		const { container, queryByText } = render(ScrOrgHome, {
			props: { orgName: 'Acme Corp', workspaces, needs: [], needsError: 'service down', stats }
		});
		expect(container.querySelector('[data-needs-error]')?.textContent).toContain('service down');
		expect(queryByText(needs[0].title)).toBeNull();
	});

	it('degrades to an honest empty state with no workspaces in jurisdiction', () => {
		const { getByText } = render(ScrOrgHome, {
			props: { orgName: 'Acme Corp', workspaces: [], needs: [], stats }
		});
		expect(getByText('No workspaces in this jurisdiction yet.')).toBeTruthy();
	});

	it('bases the phone 4px-grid rhythm and steps up at md, with no inline spacing px', () => {
		// Word-boundary matched: `toContain('p-8')` also matches inside `md:p-8`.
		const { container } = render(ScrOrgHome, {
			props: { orgName: 'Acme Corp', workspaces, needs, stats }
		});
		const el = container.querySelector('[data-component="screen-body"]')!.firstElementChild as HTMLElement;
		expect(el.className).toMatch(/\bp-4\b/);
		expect(el.className).toMatch(/\bgap-4\b/);
		expect(el.className).toMatch(/\bmd:p-8\b/);
		expect(el.className).toMatch(/\bmd:gap-6\b/);
		expect(el.className).not.toMatch(/(?<!:)\bp-8\b/);
		expect(el.getAttribute('style') ?? '').not.toMatch(/padding:|gap:/);
	});
});

describe('ScrOrgLadder — dōjō authors its own constitution by scope', () => {
	afterEach(cleanup);

	it('groups the sections into Dōjō · Workspace · Repository (D25)', () => {
		const { getByText, queryByText } = render(ScrOrgLadder, { props: { orgName: 'Acme Corp', sections } });
		expect(getByText('Constitution')).toBeTruthy();
		expect(getByText('Dōjō')).toBeTruthy();
		expect(getByText('Workspace')).toBeTruthy();
		expect(getByText('Repository')).toBeTruthy();
		// the retired rungs are gone (teams are reporting only; a stack is a pack).
		expect(queryByText('Teams')).toBeNull();
		expect(queryByText('Stacks')).toBeNull();
		// the section rail shows a scope by name.
		expect(getByText('Dōjō-wide')).toBeTruthy();
		expect(getByText('payments')).toBeTruthy();
	});

	it('shows the active (dōjō) section rules by default', () => {
		const { getByText } = render(ScrOrgLadder, { props: { orgName: 'Acme Corp', sections } });
		expect(getByText('Dōjō-wide · rules')).toBeTruthy();
		expect(getByText('No secrets in source — vault only, never .env in git')).toBeTruthy();
	});

	it('switching sections shows that section rules and the packs applied there', async () => {
		const { getByText, queryByText } = render(ScrOrgLadder, {
			props: { orgName: 'Acme Corp', sections }
		});
		await fireEvent.click(getByText('acme/web'));
		expect(getByText('acme/web · rules')).toBeTruthy();
		// the stack packs sit on the repository where the stack is detected (D33).
		expect(getByText('Rule packs applied here')).toBeTruthy();
		expect(getByText('TypeScript')).toBeTruthy();
		expect(getByText('Svelte')).toBeTruthy();
		// a dōjō rule is no longer shown.
		expect(queryByText('Never log tokens or PII, even at debug level')).toBeNull();
	});

	it('shows the honest no-pack line for a scope with no pack adopted', async () => {
		const { getByText } = render(ScrOrgLadder, { props: { orgName: 'Acme Corp', sections } });
		await fireEvent.click(getByText('acme/ledger'));
		expect(
			getByText('No pack adopted — this scope runs on its own rules and the dōjō baseline.')
		).toBeTruthy();
	});

	it('toggling a rule off excludes it and reveals the show-excluded control', async () => {
		const { getByText, getAllByLabelText, queryByText } = render(ScrOrgLadder, {
			props: { orgName: 'Acme Corp', sections }
		});
		// four dōjō rules, all included → four "Exclude rule" toggles.
		const toggles = getAllByLabelText('Exclude rule');
		expect(toggles.length).toBe(4);
		await fireEvent.click(toggles[0]);
		// the first rule is now hidden (consolidated view) and a "Show 1 excluded"
		// control appears.
		expect(queryByText('No secrets in source — vault only, never .env in git')).toBeNull();
		expect(getByText('Show 1 excluded')).toBeTruthy();
	});

	it('showing excluded reveals the hidden rule again', async () => {
		const { getByText, getAllByLabelText } = render(ScrOrgLadder, {
			props: { orgName: 'Acme Corp', sections }
		});
		await fireEvent.click(getAllByLabelText('Exclude rule')[0]);
		await fireEvent.click(getByText('Show 1 excluded'));
		expect(getByText('No secrets in source — vault only, never .env in git')).toBeTruthy();
		expect(getByText('Hide 1 excluded')).toBeTruthy();
	});

	it('opens the RuleEditor in add mode from "New rule"', async () => {
		const { getByText } = render(ScrOrgLadder, { props: { orgName: 'Acme Corp', sections } });
		await fireEvent.click(getByText('New rule'));
		// the editor overlay is open in add mode.
		expect(getByText('Add rule')).toBeTruthy();
	});

	it('opens the RuleEditor in edit mode from a rule pencil', async () => {
		const { getByText, getAllByLabelText } = render(ScrOrgLadder, {
			props: { orgName: 'Acme Corp', sections }
		});
		await fireEvent.click(getAllByLabelText('Edit rule')[0]);
		// the editor overlay is open in edit mode.
		expect(getByText('Save rule')).toBeTruthy();
	});

	it('saving the editor closes the overlay', async () => {
		const { getByText, getAllByText, queryByText } = render(ScrOrgLadder, {
			props: { orgName: 'Acme Corp', sections }
		});
		await fireEvent.click(getByText('New rule'));
		// "Add rule" appears twice while open (the section Add + the editor CTA); the
		// last is the editor's save button.
		const save = getAllByText('Add rule');
		await fireEvent.click(save[save.length - 1]);
		// the editor's family picker (unique to the overlay) is gone.
		expect(queryByText('Family')).toBeNull();
	});
});

describe('RuleEditor — compose a rule', () => {
	afterEach(cleanup);

	it('seeds add mode with an empty draft and offers the family picker', () => {
		const { getByText, getByPlaceholderText } = render(RuleEditor, {
			props: { scope: 'Payments', scopeName: 'Acme Corp' }
		});
		expect(getByText('New rule')).toBeTruthy();
		expect(getByText('Family')).toBeTruthy();
		expect((getByPlaceholderText(/State the rule/) as HTMLTextAreaElement).value).toBe('');
	});

	it('seeds edit mode from the passed rule', () => {
		const rule = { kanji: '守', text: 'Never log tokens', hard: true };
		const { getByText, getByDisplayValue } = render(RuleEditor, {
			props: { rule, scope: 'Dōjō-wide', scopeName: 'Acme Corp' }
		});
		expect(getByText('Edit rule')).toBeTruthy();
		expect(getByDisplayValue('Never log tokens')).toBeTruthy();
	});

	it('onSave bubbles the composed rule (text · family · non-negotiable)', async () => {
		const onSave = vi.fn();
		const { getByText, getByPlaceholderText } = render(RuleEditor, {
			props: { scope: 'Payments', scopeName: 'Acme Corp', onSave }
		});
		await fireEvent.input(getByPlaceholderText(/State the rule/), {
			target: { value: 'Reconcile before any migration' }
		});
		// pick the "review" family (検) and flip non-negotiable on.
		await fireEvent.click(getByText('review'));
		await fireEvent.click(getByText('Non-negotiable'));
		await fireEvent.click(getByText('Add rule'));
		expect(onSave).toHaveBeenCalledWith({
			kanji: '検',
			text: 'Reconcile before any migration',
			hard: true
		});
	});

	it('onClose fires from Cancel', async () => {
		const onClose = vi.fn();
		const { getByText } = render(RuleEditor, {
			props: { scope: 'Payments', scopeName: 'Acme Corp', onClose }
		});
		await fireEvent.click(getByText('Cancel'));
		expect(onClose).toHaveBeenCalled();
	});
});
