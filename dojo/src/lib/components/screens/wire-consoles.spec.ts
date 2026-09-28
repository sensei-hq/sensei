import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrTriage from './ScrTriage.svelte';
import ScrApprovals from './ScrApprovals.svelte';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import ScrIdentity from './ScrIdentity.svelte';
import ScrIncidents from './ScrIncidents.svelte';
import ScrClientAudit from './ScrClientAudit.svelte';
import ScrHealth from './ScrHealth.svelte';
import type { TriageRow } from '$lib/triage-data';
import type { Membership, Identity, AuditEvent, HealthRollup } from '$lib/admin-data';
import type { Incident } from '$lib/client-data';
import {
	toKitTriageGroups,
	toKitApprovals,
	toKitCandidateDetail
} from '$lib/triage-map';
import {
	toKitMembers,
	toKitRolePolicies,
	toKitAuditThread,
	toKitIdentity,
	toKitHealth
} from '$lib/admin-map';
import { toKitIncidents, toKitClientAudit } from '$lib/incidents-map';

// The Tier-2 dojo ORG CONSOLE screens wired to /v1: this drives each screen
// through the SAME wire→kit mappers the org loader uses, off MOCKED wire rows
// (TriageRow / Membership / Identity / AuditEvent / Incident / HealthRollup) —
// proving the whole wire → kit → screen path renders real-shaped data, and that
// each list-primary screen degrades to an honest-empty state when the client
// returns nothing (the DJ1 no-data / dev-404 path).

const NOW = new Date('2026-07-23T12:00:00Z');

const wireTriage: TriageRow = {
	signature: 'sig-1',
	artifact_id: 'a1',
	kind: 'pattern',
	title: 'Idempotency key on money-moving mutations',
	owner_scope: { label: 'Payments' },
	confidence: 0.95,
	contributor_count: 3,
	similarity: null,
	nearest_artifact_id: null,
	state: 'queued',
	created_at: '2026-07-22T12:00:00Z'
};

const wireMember: Membership = {
	id: 'm1',
	user_id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
	role: 'maintainer',
	kind: 'employer',
	authenticated_via: 'github_oauth',
	sync_status: 'healthy',
	attribution_default: 'named',
	last_heartbeat_at: '2026-07-23T11:00:00Z',
	disabled_at: null,
	created_at: '2026-01-01T00:00:00Z',
	display_name: null,
	email: null
};

const wireIdentity: Identity = {
	id: 'id1',
	principal_id: 'p1',
	provider: 'github_oauth',
	subject: 'gh|1',
	email: null,
	display_name: null,
	created_at: '2026-01-01T00:00:00Z',
	last_login_at: null
};

const wireAudit: AuditEvent = {
	id: 1,
	ts: '2026-07-23T11:22:00Z',
	actor_id: 'u1',
	engagement_id: 'eng-12345678',
	action: 'publish',
	target: 'idempotency pattern',
	detail: {}
};

const wireIncident: Incident = {
	id: 'i1',
	engagement_id: 'eng-12345678',
	client_name: null,
	artifact_id: null,
	title: 'Near-leak: client hostname in a shared prompt',
	description: null,
	severity: 'high',
	status: 'investigating',
	owner_id: null,
	sla_due_at: null,
	resolution: null,
	opened_at: '2026-07-20T12:00:00Z',
	resolved_at: null
};

const wireHealth: HealthRollup = { connections: 3, queue_depth: 12, publish_rate_1h: 5, error_rate_1h: 0 };

describe('ScrTriage — wired triage queue', () => {
	afterEach(cleanup);

	it('renders a candidate + its detail from mapped wire rows', () => {
		const groups = toKitTriageGroups([wireTriage], NOW);
		const detail = toKitCandidateDetail(wireTriage);
		const { getAllByText } = render(ScrTriage, {
			props: { orgName: 'Acme Corp', groups, detail }
		});
		// The candidate title appears in the list (and the detail pane).
		expect(getAllByText('Idempotency key on money-moving mutations').length).toBeGreaterThan(0);
		// The scope label heads the group (and appears in the detail context).
		expect(getAllByText('Payments').length).toBeGreaterThan(0);
	});

	it('shows the honest-empty state when the queue is clear', () => {
		const { getByText } = render(ScrTriage, {
			props: { orgName: 'Acme Corp', groups: toKitTriageGroups([], NOW), detail: toKitCandidateDetail(undefined) }
		});
		expect(getByText('The triage queue is clear.')).toBeTruthy();
	});
});

describe('ScrApprovals — wired second-approval queue', () => {
	afterEach(cleanup);

	it('renders a high-impact candidate as an approval row', () => {
		const approvals = toKitApprovals([wireTriage], NOW); // conf 0.95 → high
		const { getByText } = render(ScrApprovals, { props: { orgName: 'Acme Corp', approvals } });
		expect(getByText('Idempotency key on money-moving mutations')).toBeTruthy();
	});

	it('shows the honest-empty state with no high-impact candidates', () => {
		const lo: TriageRow = { ...wireTriage, confidence: 0.5 };
		const { getByText } = render(ScrApprovals, {
			props: { orgName: 'Acme Corp', approvals: toKitApprovals([lo], NOW) }
		});
		expect(getByText('Nothing awaiting a second look.')).toBeTruthy();
	});
});

describe('ScrRoleSurfaces — wired members / audit', () => {
	afterEach(cleanup);

	it('renders a member row + the role ladder from mapped wire rows', () => {
		const { getByText } = render(ScrRoleSurfaces, {
			props: {
				orgName: 'Acme Corp',
				tab: 'members',
				members: toKitMembers([wireMember], { now: NOW }),
				policies: toKitRolePolicies([]),
				audit: toKitAuditThread([wireAudit], NOW)
			}
		});
		expect(getByText('aaaaaaaa')).toBeTruthy(); // short-id fallback name
	});

	it('renders the audit thread on the audit tab', () => {
		const { getByText } = render(ScrRoleSurfaces, {
			props: {
				orgName: 'Acme Corp',
				tab: 'audit',
				members: toKitMembers([], { now: NOW }),
				policies: toKitRolePolicies([]),
				audit: toKitAuditThread([wireAudit], NOW)
			}
		});
		expect(getByText(/publish · idempotency pattern/)).toBeTruthy();
	});
});

describe('ScrIdentity — wired identity mappings', () => {
	afterEach(cleanup);

	it('renders the IdP header + a mapping from mapped wire identities', () => {
		const identity = toKitIdentity([wireIdentity, wireIdentity]);
		const { getAllByText } = render(ScrIdentity, { props: { orgName: 'Acme Corp', identity } });
		expect(getAllByText('GitHub').length).toBeGreaterThan(0);
	});

	it('reads not-connected with no identities', () => {
		const { getByText } = render(ScrIdentity, {
			props: { orgName: 'Acme Corp', identity: toKitIdentity([]) }
		});
		expect(getByText('not connected')).toBeTruthy();
	});
});

describe('ScrIncidents — wired incidents', () => {
	afterEach(cleanup);

	it('renders an incident row from a mapped wire incident', () => {
		const incidents = toKitIncidents([wireIncident], NOW);
		const { getByText } = render(ScrIncidents, { props: { orgName: 'Acme Corp', incidents } });
		expect(getByText(/Near-leak/)).toBeTruthy();
	});

	it('shows the honest-empty state with no incidents', () => {
		const { getByText } = render(ScrIncidents, {
			props: { orgName: 'Acme Corp', incidents: toKitIncidents([], NOW) }
		});
		expect(getByText('No incidents on record.')).toBeTruthy();
	});
});

describe('ScrClientAudit — wired ledger', () => {
	afterEach(cleanup);

	it('renders a ledger row from a mapped audit event', () => {
		const entries = toKitClientAudit([wireAudit], NOW);
		const { getByText } = render(ScrClientAudit, { props: { orgName: 'Acme Corp', entries } });
		expect(getByText('publish')).toBeTruthy();
	});

	it('shows the honest-empty state with no entries', () => {
		const { getByText } = render(ScrClientAudit, {
			props: { orgName: 'Acme Corp', entries: toKitClientAudit([], NOW) }
		});
		expect(getByText('The ledger is empty.')).toBeTruthy();
	});
});

describe('ScrHealth — wired rollup', () => {
	afterEach(cleanup);

	it('renders the vital-signs cards from the mapped rollup', () => {
		const health = toKitHealth(wireHealth);
		const { getByText } = render(ScrHealth, { props: { orgName: 'Acme Corp', health } });
		expect(getByText('Live connections')).toBeTruthy();
		expect(getByText('Queue depth')).toBeTruthy();
	});

	it('renders zeroed cards from an empty rollup (honest empty)', () => {
		const health = toKitHealth({ connections: 0, queue_depth: 0, publish_rate_1h: 0, error_rate_1h: 0 });
		const { getByText } = render(ScrHealth, { props: { orgName: 'Acme Corp', health } });
		expect(getByText('all healthy')).toBeTruthy();
	});
});
