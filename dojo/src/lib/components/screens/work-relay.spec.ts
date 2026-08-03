import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrProjects from './ScrProjects.svelte';
import ScrProjectPreview from './ScrProjectPreview.svelte';
import { projects, ladder, conflicts } from '$lib/components/kit/fixtures';

// Chunk-2 personal Work screens. Each renders off the kit fixtures
// (presentational — real /v1 wiring is a later chunk). We assert rows render,
// empty states degrade honestly, the preview toggle + ladder/conflicts render,
// and the project drill-in opens the preview.

describe('ScrProjects — the full project list', () => {
	afterEach(cleanup);

	it('renders a row per project off fixtures', () => {
		const { getByText, getAllByText } = render(ScrProjects, { props: { projects } });
		expect(getByText('Projects')).toBeTruthy();
		expect(getByText('lumen-auth')).toBeTruthy();
		expect(getAllByText(/ledger-core/).length).toBeGreaterThan(0);
		expect(getByText('personal-site')).toBeTruthy();
	});

	it('fires onOpenProject when a row is clicked', async () => {
		const onOpenProject = vi.fn();
		const { getByText } = render(ScrProjects, { props: { projects, onOpenProject } });
		await fireEvent.click(getByText('lumen-auth'));
		expect(onOpenProject).toHaveBeenCalledWith(projects[0]);
	});

	it('shows an honest empty state with no projects', () => {
		const { getByText, queryByText } = render(ScrProjects, { props: { projects: [] } });
		expect(getByText('No projects yet.')).toBeTruthy();
		expect(queryByText('lumen-auth')).toBeNull();
	});

	it('shows an ERROR state (distinct from empty) when the read failed', () => {
		const { getByText, queryByText } = render(ScrProjects, {
			props: { projects: [], error: 'could not reach the dojo service' }
		});
		// The failure surfaces — never the honest-empty state masking a read error.
		expect(getByText("Couldn't load your projects.")).toBeTruthy();
		expect(getByText(/could not reach the dojo service/)).toBeTruthy();
		expect(queryByText('No projects yet.')).toBeNull();
	});
});

describe('ScrProjectPreview — the resolved-constitution drill-in', () => {
	afterEach(cleanup);

	const company = projects[0]; // lumen-auth (company)
	const personal = projects[2]; // personal-site (personal)

	it('renders the project header, classification banner and ladder', () => {
		const { getByText, getAllByText } = render(ScrProjectPreview, {
			props: { project: company, ladder, conflicts }
		});
		// the header title is the project name.
		expect(getAllByText(/lumen-auth/).length).toBeGreaterThan(0);
		// the ladder rungs render (Company scope label).
		expect(getAllByText('Company').length).toBeGreaterThan(0);
	});

	it('shows the discarded-conflicts section for a company project', () => {
		const { getByText } = render(ScrProjectPreview, {
			props: { project: company, ladder, conflicts }
		});
		expect(getByText('Discarded by the ladder')).toBeTruthy();
		// a conflict topic renders.
		expect(getByText(conflicts[0].topic)).toBeTruthy();
	});

	it('hides the discarded section for a personal project', () => {
		const { queryByText } = render(ScrProjectPreview, {
			props: { project: personal, ladder, conflicts }
		});
		expect(queryByText('Discarded by the ladder')).toBeNull();
	});

	it('client banner uses the universal (always-on) dereference wording (F4 Q4)', () => {
		const client = { ...company, classification: 'client' };
		const { getByText } = render(ScrProjectPreview, { props: { project: client, ladder, conflicts } });
		expect(getByText(/dereferenced, as they always are/)).toBeTruthy();
	});

	it('shows an honest "resolves in your editor" state (no fabricated ladder) when the ladder is not federated', () => {
		const { getByText, queryByText } = render(ScrProjectPreview, {
			props: { project: company, ladder: [], conflicts: [] }
		});
		// Real project header still renders.
		expect(getByText(company.name)).toBeTruthy();
		// The honest-empty constitution state — NOT a fabricated ladder/toggle.
		expect(getByText(/resolves in your editor/i)).toBeTruthy();
		expect(queryByText('By layer')).toBeNull();
		expect(queryByText('The ladder — broad → specific')).toBeNull();
		expect(queryByText('Discarded by the ladder')).toBeNull();
	});

	it('toggles between by-layer and consolidated views', async () => {
		const { getByText, queryByText } = render(ScrProjectPreview, {
			props: { project: company, ladder, conflicts }
		});
		// opens on the by-layer view.
		expect(getByText('The ladder — broad → specific')).toBeTruthy();
		await fireEvent.click(getByText('Consolidated'));
		expect(getByText('Consolidated constitution')).toBeTruthy();
		expect(queryByText('The ladder — broad → specific')).toBeNull();
	});

	it('fires onBack from the back header', async () => {
		const onBack = vi.fn();
		const { getByText } = render(ScrProjectPreview, {
			props: { project: company, ladder, conflicts, onBack }
		});
		await fireEvent.click(getByText('Back to projects'));
		expect(onBack).toHaveBeenCalled();
	});
});
