import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrTriage from './ScrTriage.svelte';
import ScrApprovals from './ScrApprovals.svelte';
import ScrKnowledge from './ScrKnowledge.svelte';
import {
	triageGroupsFor,
	candidateDetailFor,
	approvalsFor,
	knowledgeFor
} from '$lib/components/kit/fixtures';

// Chunk-5 maintainer (Govern) role consoles (the /org/[slug]/[section] role
// screens). Each renders off the ported console fixtures (presentational — real
// /v1 wiring is a later chunk). We assert the triage queue groups + ranks +
// selects, the approvals queue rows + its empty state, and the knowledge library
// sections + prune policy.

const groups = triageGroupsFor('acme');
const detail = candidateDetailFor('acme');
const approvals = approvalsFor('acme');
const knowledge = knowledgeFor('acme');

describe('ScrTriage — scope-grouped, confidence-ranked candidate queue', () => {
	afterEach(cleanup);

	it('heads the govern console with the total candidate count', () => {
		const { getByText } = render(ScrTriage, { props: { orgName: 'Acme Corp', groups, detail } });
		expect(getByText('Acme Corp · govern')).toBeTruthy();
		expect(getByText('Triage')).toBeTruthy();
		// 5 candidates across the three scope groups.
		expect(getByText('5')).toBeTruthy();
	});

	it('groups candidates by scope and shows a candidate + its origin', () => {
		const { getByText, getAllByText } = render(ScrTriage, {
			props: { orgName: 'Acme Corp', groups, detail }
		});
		expect(getByText('Payments')).toBeTruthy();
		expect(getByText('React · TypeScript')).toBeTruthy();
		expect(getByText('Auth boundary')).toBeTruthy();
		// the default-selected candidate's title shows in the queue row AND the
		// detail-pane header, so it appears twice.
		expect(getAllByText('Idempotency key on every money-moving mutation').length).toBe(2);
		expect(getByText('6 sessions · 3 repos')).toBeTruthy();
	});

	it('shows the first candidate detail (learning + evidence) in the pane by default', () => {
		const { getByText } = render(ScrTriage, { props: { orgName: 'Acme Corp', groups, detail } });
		expect(getByText(detail.learning)).toBeTruthy();
		expect(getByText('· ' + detail.evidence[0])).toBeTruthy();
	});

	it('a high-impact candidate with a conflict shows the second-approval note', () => {
		const { getByText } = render(ScrTriage, { props: { orgName: 'Acme Corp', groups, detail } });
		// t1 (default selection) is high impact → the note is present; it also has a
		// conflict → the conflict block shows the winning rule.
		expect(getByText('Approving sends this to a second maintainer before it publishes.')).toBeTruthy();
		expect(getByText(detail.conflict.winner)).toBeTruthy();
	});

	it('selecting another candidate moves the detail-pane focus to it', async () => {
		const { getAllByText, queryByText } = render(ScrTriage, {
			props: { orgName: 'Acme Corp', groups, detail }
		});
		// the queue row for a normal-impact candidate (before selection its title is
		// only in the queue, so a single match — the row button).
		const row = getAllByText('Reconcile before any ledger migration')[0].closest('button');
		await fireEvent.click(row!);
		// the row is now the selected one (aria-pressed) and — being normal impact —
		// the second-approval note is gone.
		expect(row?.getAttribute('aria-pressed')).toBe('true');
		expect(
			queryByText('Approving sends this to a second maintainer before it publishes.')
		).toBeNull();
	});

	it('degrades to an honest empty state when the queue is clear', () => {
		const { getByText } = render(ScrTriage, {
			props: { orgName: 'Acme Corp', groups: [], detail }
		});
		expect(getByText('The triage queue is clear.')).toBeTruthy();
	});

	it('uses the desktop 4px-grid rhythm with no inline spacing px on the wrapper', () => {
		const { container } = render(ScrTriage, { props: { orgName: 'Acme Corp', groups, detail } });
		const el = container.querySelector('[data-component="screen-body"]')!.firstElementChild as HTMLElement;
		expect(el.className).toContain('p-8');
		expect(el.className).toContain('gap-6');
		expect(el.getAttribute('style') ?? '').not.toMatch(/padding:|gap:/);
	});
});

describe('ScrApprovals — second-approval queue', () => {
	afterEach(cleanup);

	it('heads the queue and lists an approval with its first approver', () => {
		const { getByText } = render(ScrApprovals, { props: { orgName: 'Acme Corp', approvals } });
		expect(getByText('Approvals')).toBeTruthy();
		expect(getByText('Never log refresh tokens, even at debug level')).toBeTruthy();
		expect(getByText('Company · first approval: Keiko Tanaka · 2h')).toBeTruthy();
	});

	it('fires onApprove / onReview with the approval', async () => {
		const onApprove = vi.fn();
		const onReview = vi.fn();
		const { getAllByText } = render(ScrApprovals, {
			props: { orgName: 'Acme Corp', approvals, onApprove, onReview }
		});
		await fireEvent.click(getAllByText('Approve')[0]);
		expect(onApprove).toHaveBeenCalledWith(approvals[0]);
		await fireEvent.click(getAllByText('Review')[0]);
		expect(onReview).toHaveBeenCalledWith(approvals[0]);
	});

	it('degrades to an honest empty state when nothing awaits a second look', () => {
		const { getByText } = render(ScrApprovals, {
			props: { orgName: 'Acme Corp', approvals: [] }
		});
		expect(getByText('Nothing awaiting a second look.')).toBeTruthy();
	});
});

describe('ScrKnowledge — published library + prune policy + catalog', () => {
	afterEach(cleanup);

	it('shows the prune-policy readout in the header', () => {
		const { getByText } = render(ScrKnowledge, { props: { orgName: 'Acme Corp', knowledge } });
		expect(getByText('Knowledge')).toBeTruthy();
		expect(getByText('Prune after 90 days unused')).toBeTruthy();
	});

	it('groups Active / Pending prune / Catalog and shows a row from each', () => {
		const { getByText } = render(ScrKnowledge, { props: { orgName: 'Acme Corp', knowledge } });
		expect(getByText('Active')).toBeTruthy();
		expect(getByText('Pending prune')).toBeTruthy();
		expect(getByText('Catalog · skills, agents & commands')).toBeTruthy();
		expect(getByText('Idempotency key on money-moving mutations')).toBeTruthy();
		expect(getByText('Deprecation window of two minor versions')).toBeTruthy();
		expect(getByText('integration-test author')).toBeTruthy();
	});
});
