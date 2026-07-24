import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrRelayWatch from './ScrRelayWatch.svelte';
import ScrRelayApprove from './ScrRelayApprove.svelte';
import ScrRelayDecide from './ScrRelayDecide.svelte';
import ScrRelayChat from './ScrRelayChat.svelte';
import ScrEngagements from './ScrEngagements.svelte';
import ScrMyDojos from './ScrMyDojos.svelte';
import type { RelayRun, RelayGate, RelaySegment } from '$lib/relay-data';
import type { Engagement } from '$lib/client-data';
import type { DojoOrg } from '$lib/dojo-data';
import {
	toKitRuns,
	toKitGates,
	toKitDecisions,
	toKitChatThread
} from '$lib/dojo2-relay-map';
import { toKitEngagements } from '$lib/dojo2-client-map';
import { toKitDojos } from '$lib/dojo2-chrome';
import { confidentialityFor } from '$lib/components/kit/fixtures';

// The Tier-1 dojo2 screens wired to /v1: this drives each screen through the SAME
// wire→kit mappers the loaders use, off MOCKED wire rows (RelayRun / RelayGate /
// RelaySegment / Engagement / DojoOrg) — proving the whole wire → kit → screen
// path renders real-shaped data, and that each screen degrades to an honest-empty
// state when the client returns nothing (the DJ1 solo / dev-404 path). The
// confidentiality panel stays on the fixture (its route isn't built); ScrMyDojos
// reuses the layout memberships mapper (toKitDojos), not a new /v1 call.

const NOW = new Date('2026-07-23T12:00:00Z');

const wireRun: RelayRun = {
	id: 'sess-1',
	run_id: 'run-1',
	title: 'lumen-auth',
	goal: 'refactor refresh-token rotation',
	status: 'running',
	progress_done: 12,
	progress_total: 30,
	current_phase: 'Phase 2',
	current_feature: 'token-rotation',
	last_event_at: '2026-07-23T11:30:00Z',
	paused_until: null,
	pause_reason: null,
	started_at: '2026-07-23T11:22:00Z',
	completed_at: null
};

const approvalGate: RelayGate = {
	id: 'g1',
	seq: 1,
	run_id: 'run-1',
	run_title: 'lumen-auth',
	segment_id: null,
	kind: 'approval',
	payload: { command: 'pnpm db:migrate --env=staging', reason: 'touches auth schema', gate_severity: 'blocking' },
	created_at: '2026-07-23T11:57:00Z'
};

const decisionGate: RelayGate = {
	id: 'd1',
	seq: 2,
	run_id: 'run-1',
	run_title: 'globex-portal',
	segment_id: null,
	kind: 'decision',
	payload: {
		prompt: 'adopt “verify webhook signature” as a client guard',
		options: ['adopt', 'keep as note', 'decline'],
		context: '4 sessions · confidence 0.91'
	},
	created_at: '2026-07-23T11:00:00Z'
};

const wireSegment: RelaySegment = {
	id: 's1',
	parent_id: null,
	seq: 1,
	title: 'Rotate refresh tokens',
	summary: 'route the debug line through the redaction sink',
	detail: null,
	state: 'active',
	is_gate: false,
	gate_severity: null,
	response_verdict: null,
	response_note: null,
	submitted_at: '2026-07-23T11:22:00Z'
};

const wireEngagement: Engagement = {
	id: 'e1',
	client: 'Globex',
	description: null,
	project_bindings: [{ project_id: 'p1', name: 'globex-portal' }],
	policy_overrides: {},
	status: 'active',
	starts_on: '2026-07-16',
	ends_on: null,
	created_at: '2026-01-01T00:00:00Z',
	updated_at: '2026-07-01T00:00:00Z'
};

const wireOrg: DojoOrg = {
	id: 'acme',
	kanji: '社',
	name: 'Acme Corp',
	kind: 'Employer',
	host: 'saas',
	url: 'github/acme',
	role: 'Admin',
	from: 'member · admin',
	members: 48,
	pending: 4
};

describe('ScrRelayWatch — wired runs', () => {
	afterEach(cleanup);

	it('renders a run card from a mapped wire run', () => {
		const runs = toKitRuns([wireRun], [approvalGate], NOW);
		const { getByText } = render(ScrRelayWatch, { props: { runs } });
		expect(getByText('Live runs')).toBeTruthy();
		expect(getByText('refactor refresh-token rotation')).toBeTruthy();
	});

	it('shows the honest-empty state when the client returns no runs', () => {
		const { getByText } = render(ScrRelayWatch, { props: { runs: toKitRuns([], [], NOW) } });
		expect(getByText('No sessions running.')).toBeTruthy();
	});
});

describe('ScrRelayApprove — wired gates', () => {
	afterEach(cleanup);

	it('renders a command card from a mapped approval gate', () => {
		const gates = toKitGates([approvalGate, decisionGate], NOW); // decision filtered out
		const { getByText } = render(ScrRelayApprove, { props: { gates } });
		expect(getByText(/pnpm db:migrate --env=staging/)).toBeTruthy();
	});

	it('fires onApprove / onDeny with the mapped gate', async () => {
		const gates = toKitGates([approvalGate], NOW);
		const onApprove = vi.fn();
		const onDeny = vi.fn();
		const { getAllByText } = render(ScrRelayApprove, { props: { gates, onApprove, onDeny } });
		await fireEvent.click(getAllByText('Approve once')[0]);
		expect(onApprove).toHaveBeenCalledWith(gates[0]);
		await fireEvent.click(getAllByText('Deny')[0]);
		expect(onDeny).toHaveBeenCalledWith(gates[0]);
	});

	it('shows the honest-empty state when no gates', () => {
		const { getByText } = render(ScrRelayApprove, { props: { gates: toKitGates([], NOW) } });
		expect(getByText('Nothing waiting on you.')).toBeTruthy();
	});
});

describe('ScrRelayDecide — wired decisions', () => {
	afterEach(cleanup);

	it('renders a decision card from a mapped decision gate', () => {
		const decisions = toKitDecisions([decisionGate, approvalGate], NOW); // approval filtered out
		const { getByText } = render(ScrRelayDecide, { props: { decisions } });
		expect(getByText('adopt “verify webhook signature” as a client guard')).toBeTruthy();
	});

	it('fires onChoose with the mapped decision and the chosen option', async () => {
		const decisions = toKitDecisions([decisionGate], NOW);
		const onChoose = vi.fn();
		const { getByText } = render(ScrRelayDecide, { props: { decisions, onChoose } });
		await fireEvent.click(getByText('adopt'));
		expect(onChoose).toHaveBeenCalledWith(decisions[0], 'adopt');
	});

	it('renders the quiet "that\'s everything" banner with no decisions', () => {
		const { getByText } = render(ScrRelayDecide, { props: { decisions: toKitDecisions([], NOW) } });
		expect(getByText("That's everything.")).toBeTruthy();
	});
});

describe('ScrRelayChat — wired thread', () => {
	afterEach(cleanup);

	it('renders a sensei turn from a mapped segment', () => {
		const thread = toKitChatThread([wireSegment], NOW);
		const { getByText } = render(ScrRelayChat, { props: { thread } });
		expect(getByText(/Rotate refresh tokens/)).toBeTruthy();
	});

	it('fires onSend with a trimmed reply (when a run is in flight)', async () => {
		const onSend = vi.fn();
		// the reply composer only shows for an active run (a non-empty thread) — an
		// empty thread degrades to the shared EmptyState with no input.
		const { getByPlaceholderText, getByLabelText } = render(ScrRelayChat, {
			props: { thread: toKitChatThread([wireSegment], NOW), onSend }
		});
		await fireEvent.input(getByPlaceholderText('reply to sensei…'), { target: { value: '  ship it  ' } });
		await fireEvent.click(getByLabelText('Send reply'));
		expect(onSend).toHaveBeenCalledWith('ship it');
	});

	it('renders the shared empty state (no composer) with no active run', () => {
		const { getByText, queryByPlaceholderText } = render(ScrRelayChat, {
			props: { thread: toKitChatThread([], NOW) }
		});
		expect(getByText('No active session.')).toBeTruthy();
		expect(queryByPlaceholderText('reply to sensei…')).toBeNull();
	});
});

describe('ScrEngagements — wired engagements', () => {
	afterEach(cleanup);

	it('renders an engagement row from a mapped wire engagement', () => {
		const engagements = toKitEngagements([wireEngagement], NOW);
		const { getByText } = render(ScrEngagements, {
			props: { orgName: 'Acme Corp', engagements, confidentiality: confidentialityFor('acme') }
		});
		expect(getByText('Globex')).toBeTruthy();
		expect(getByText(/globex-portal/)).toBeTruthy();
		// the confidentiality panel (still fixture-backed) renders alongside.
		expect(getByText('What crosses the boundary')).toBeTruthy();
	});

	it('renders an empty register (count 0) when the client returns none', () => {
		const { getByText, queryByText } = render(ScrEngagements, {
			props: { orgName: 'Acme Corp', engagements: toKitEngagements([], NOW), confidentiality: confidentialityFor('acme') }
		});
		expect(getByText('Engagements')).toBeTruthy();
		expect(queryByText('Globex')).toBeNull();
	});
});

describe('ScrMyDojos — wired memberships', () => {
	afterEach(cleanup);

	it('renders a membership row from the mapped layout memberships', () => {
		const dojos = toKitDojos([wireOrg]);
		const { getByText } = render(ScrMyDojos, { props: { dojos } });
		expect(getByText('Acme Corp')).toBeTruthy();
	});

	it('shows the honest-empty state with no memberships (solo user, DJ1)', () => {
		const { getByText } = render(ScrMyDojos, { props: { dojos: toKitDojos([]) } });
		expect(getByText('No memberships yet — create or join a Dōjō.')).toBeTruthy();
	});
});
