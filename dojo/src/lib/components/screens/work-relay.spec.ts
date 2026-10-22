import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrWorkspaces from './ScrWorkspaces.svelte';
import ScrWorkspacePreview from './ScrWorkspacePreview.svelte';
import { workspaces, ladder, conflicts } from '$lib/components/kit/fixtures';

// Chunk-2 personal Work screens. Each renders off the kit fixtures
// (presentational — real /v1 wiring is a later chunk). We assert rows render,
// empty states degrade honestly, the preview toggle + ladder/conflicts render,
// and the workspace drill-in opens the preview.

describe('ScrWorkspaces — the full workspace list', () => {
	afterEach(cleanup);

	it('renders a row per workspace off fixtures', () => {
		const { getByText, getAllByText } = render(ScrWorkspaces, { props: { workspaces } });
		expect(getByText('Workspaces')).toBeTruthy();
		expect(getByText('lumen-auth')).toBeTruthy();
		expect(getAllByText(/ledger-core/).length).toBeGreaterThan(0);
		expect(getByText('personal-site')).toBeTruthy();
	});

	it('fires onOpenWorkspace when a row is clicked', async () => {
		const onOpenWorkspace = vi.fn();
		const { getByText } = render(ScrWorkspaces, { props: { workspaces, onOpenWorkspace } });
		await fireEvent.click(getByText('lumen-auth'));
		expect(onOpenWorkspace).toHaveBeenCalledWith(workspaces[0]);
	});

	it('shows an honest empty state with no workspaces', () => {
		const { getByText, queryByText } = render(ScrWorkspaces, { props: { workspaces: [] } });
		expect(getByText('No workspaces yet.')).toBeTruthy();
		expect(queryByText('lumen-auth')).toBeNull();
	});

	it('shows an ERROR state (distinct from empty) when the read failed', () => {
		const { getByText, queryByText } = render(ScrWorkspaces, {
			props: { workspaces: [], error: 'could not reach the dojo service' }
		});
		// The failure surfaces — never the honest-empty state masking a read error.
		expect(getByText("Couldn't load your workspaces.")).toBeTruthy();
		expect(getByText(/could not reach the dojo service/)).toBeTruthy();
		expect(queryByText('No workspaces yet.')).toBeNull();
	});
});

describe('ScrWorkspacePreview — the resolved-constitution drill-in', () => {
	afterEach(cleanup);

	const company = workspaces[0]; // lumen-auth (company)
	const personal = workspaces[2]; // personal-site (personal)

	it('renders the workspace header, classification banner and ladder', () => {
		const { getByText, getAllByText } = render(ScrWorkspacePreview, {
			props: { workspace: company, ladder, conflicts }
		});
		// the header title is the workspace name.
		expect(getAllByText(/lumen-auth/).length).toBeGreaterThan(0);
		// the ladder rungs render: dōjō → workspace → repository (D25).
		expect(getAllByText('Dōjō').length).toBeGreaterThan(0);
		expect(getAllByText('Workspace').length).toBeGreaterThan(0);
		expect(getAllByText('Repository').length).toBeGreaterThan(0);
	});

	it('shows the packs applied at each rung — the stack packs on the repository (D33)', () => {
		const { getByText, queryByText } = render(ScrWorkspacePreview, {
			props: { workspace: company, ladder, conflicts }
		});
		expect(getByText('TypeScript')).toBeTruthy();
		expect(getByText('Svelte')).toBeTruthy();
		expect(getByText('OWASP baseline')).toBeTruthy();
		// no retired rung renders.
		expect(queryByText('Stack')).toBeNull();
		expect(queryByText('Client')).toBeNull();
		expect(queryByText('Team')).toBeNull();
	});

	it('shows the discarded-conflicts section for a company workspace', () => {
		const { getByText } = render(ScrWorkspacePreview, {
			props: { workspace: company, ladder, conflicts }
		});
		expect(getByText('Discarded by the ladder')).toBeTruthy();
		// a conflict topic renders.
		expect(getByText(conflicts[0].topic)).toBeTruthy();
	});

	it('hides the discarded section for a personal workspace', () => {
		const { queryByText } = render(ScrWorkspacePreview, {
			props: { workspace: personal, ladder, conflicts }
		});
		expect(queryByText('Discarded by the ladder')).toBeNull();
	});

	it('client banner uses the universal (always-on) dereference wording (F4 Q4)', () => {
		const client = { ...company, classification: 'client' };
		const { getByText } = render(ScrWorkspacePreview, { props: { workspace: client, ladder, conflicts } });
		expect(getByText(/dereferenced, as they always are/)).toBeTruthy();
	});

	it('shows an honest "resolves in your editor" state (no fabricated ladder) when the ladder is not federated', () => {
		const { getByText, queryByText } = render(ScrWorkspacePreview, {
			props: { workspace: company, ladder: [], conflicts: [] }
		});
		// Real workspace header still renders.
		expect(getByText(company.name)).toBeTruthy();
		// The honest-empty constitution state — NOT a fabricated ladder/toggle.
		expect(getByText(/resolves in your editor/i)).toBeTruthy();
		expect(queryByText('By layer')).toBeNull();
		expect(queryByText('The ladder — broad → specific')).toBeNull();
		expect(queryByText('Discarded by the ladder')).toBeNull();
	});

	it('toggles between by-layer and consolidated views', async () => {
		const { getByText, queryByText } = render(ScrWorkspacePreview, {
			props: { workspace: company, ladder, conflicts }
		});
		// opens on the by-layer view.
		expect(getByText('The ladder — broad → specific')).toBeTruthy();
		await fireEvent.click(getByText('Consolidated'));
		expect(getByText('Consolidated constitution')).toBeTruthy();
		expect(queryByText('The ladder — broad → specific')).toBeNull();
	});

	it('fires onBack from the back header', async () => {
		const onBack = vi.fn();
		const { getByText } = render(ScrWorkspacePreview, {
			props: { workspace: company, ladder, conflicts, onBack }
		});
		await fireEvent.click(getByText('Back to workspaces'));
		expect(onBack).toHaveBeenCalled();
	});
});
