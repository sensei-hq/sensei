import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrProjects from './ScrProjects.svelte';
import ScrProjectPreview from './ScrProjectPreview.svelte';
import ScrRelayWatch from './ScrRelayWatch.svelte';
import ScrRelayApprove from './ScrRelayApprove.svelte';
import ScrRelayDecide from './ScrRelayDecide.svelte';
import ScrRelayChat from './ScrRelayChat.svelte';
import { projects, runs, gates, decisions, ladder, conflicts, chat, me } from '$lib/components/kit/fixtures';

// Chunk-2 personal Work + Relay screens. Each renders off the kit fixtures
// (presentational — real /v1 wiring is a later chunk). We assert rows render,
// empty states degrade honestly, the preview toggle + ladder/conflicts render,
// relay actions fire callbacks, and the project drill-in opens the preview.

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

describe('ScrRelayWatch — live runs', () => {
	afterEach(cleanup);

	it('renders a run card per fixture run', () => {
		const { getByText } = render(ScrRelayWatch, { props: { runs } });
		expect(getByText('Live runs')).toBeTruthy();
		expect(getByText('refactor refresh-token rotation')).toBeTruthy();
	});

	it('shows an honest empty state with no runs', () => {
		const { getByText } = render(ScrRelayWatch, { props: { runs: [] } });
		expect(getByText('No sessions running.')).toBeTruthy();
	});
});

describe('ScrRelayApprove — commands awaiting approval', () => {
	afterEach(cleanup);

	it('renders a gate card per fixture gate', () => {
		const { getByText } = render(ScrRelayApprove, { props: { gates } });
		expect(getByText('Commands waiting on you')).toBeTruthy();
		expect(getByText(/pnpm db:migrate --env=staging/)).toBeTruthy();
	});

	it('fires onApprove / onDeny with the gate', async () => {
		const onApprove = vi.fn();
		const onDeny = vi.fn();
		const { getAllByText } = render(ScrRelayApprove, { props: { gates, onApprove, onDeny } });
		await fireEvent.click(getAllByText('Approve once')[0]);
		expect(onApprove).toHaveBeenCalledWith(gates[0]);
		await fireEvent.click(getAllByText('Deny')[0]);
		expect(onDeny).toHaveBeenCalledWith(gates[0]);
	});

	it('shows an honest empty state with no gates', () => {
		const { getByText } = render(ScrRelayApprove, { props: { gates: [] } });
		expect(getByText('Nothing waiting on you.')).toBeTruthy();
	});
});

describe('ScrRelayDecide — rules to sign off', () => {
	afterEach(cleanup);

	it('renders a decision card per fixture decision', () => {
		const { getByText } = render(ScrRelayDecide, { props: { decisions } });
		expect(getByText('Rules to sign off')).toBeTruthy();
		expect(getByText('adopt ‘verify webhook signature’ as a client guard')).toBeTruthy();
	});

	it('fires onChoose with the decision and the chosen option', async () => {
		const onChoose = vi.fn();
		const { getByText } = render(ScrRelayDecide, { props: { decisions, onChoose } });
		await fireEvent.click(getByText('adopt to Client rung'));
		expect(onChoose).toHaveBeenCalledWith(decisions[0], 'adopt to Client rung');
	});

	it('shows the quiet empty banner when nothing is pending', () => {
		const { getByText } = render(ScrRelayDecide, { props: { decisions: [] } });
		expect(getByText("That's everything.")).toBeTruthy();
	});
});

describe('ScrRelayChat — sensei chat thread', () => {
	afterEach(cleanup);

	it('renders the thread and a reply input', () => {
		const { getByText, getByPlaceholderText } = render(ScrRelayChat, {
			props: { thread: chat, me, project: 'lumen-auth', session: 's-2891' }
		});
		expect(getByText(/Noticed the refresh-token rotation touches the logger/)).toBeTruthy();
		expect(getByPlaceholderText('reply to sensei…')).toBeTruthy();
	});

	it('fires onSend with the typed reply and clears the input', async () => {
		const onSend = vi.fn();
		const { getByPlaceholderText, getByLabelText } = render(ScrRelayChat, {
			props: { thread: chat, me, onSend }
		});
		const input = getByPlaceholderText('reply to sensei…') as HTMLInputElement;
		await fireEvent.input(input, { target: { value: 'Approving now.' } });
		await fireEvent.click(getByLabelText('Send reply'));
		expect(onSend).toHaveBeenCalledWith('Approving now.');
		expect(input.value).toBe('');
	});

	it('does not send an empty reply', async () => {
		const onSend = vi.fn();
		const { getByLabelText } = render(ScrRelayChat, { props: { thread: chat, me, onSend } });
		await fireEvent.click(getByLabelText('Send reply'));
		expect(onSend).not.toHaveBeenCalled();
	});
});
