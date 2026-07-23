// Test fixtures for the dojo2 kit specs — lifted from the mockup mock data
// (docs/mockups/Sensei/lib/data/dojo2-data.js window.DOJO2) so the specs render
// the components against the exact shapes the screens will bind to.

import type {
	KitProject,
	KitDojo,
	KitMe,
	KitOrg,
	KitNavGroup,
	KitNavItem,
	KitLadderRung,
	KitConflict,
	KitStanceDial,
	KitRun,
	KitGate,
	KitNeed,
	KitDecision,
	KitChatTurn
} from './types';

export const me: KitMe = { name: 'Rin Saito', handle: 'rin', avatar: 'R' };

export const dojos: KitDojo[] = [
	{
		slug: 'acme',
		kanji: '社',
		name: 'Acme Corp',
		kind: 'employer',
		role: 'admin',
		route: 'sensei-hq.com/acme',
		members: 48,
		projects: 9,
		needs: 4
	},
	{
		slug: 'globex',
		kanji: '客',
		name: 'Globex',
		kind: 'client',
		role: 'lead',
		route: 'sensei-hq.com/globex',
		members: 12,
		projects: 3,
		needs: 2
	},
	{
		slug: 'rustco',
		kanji: '群',
		name: 'Rust Guild',
		kind: 'community',
		role: 'developer',
		route: 'sensei-hq.com/rust-guild',
		members: 340,
		projects: 18,
		needs: 0
	}
];

export const org: KitOrg = {
	slug: 'acme',
	kanji: '社',
	name: 'Acme Corp',
	kind: 'employer',
	role: 'admin',
	route: 'sensei-hq.com/acme'
};

export const projects: KitProject[] = [
	{
		id: 'auth',
		name: 'lumen-auth',
		repo: 'acme/lumen-auth',
		dojoName: 'Acme Corp',
		classification: 'company',
		phase: 'notice',
		lastRun: '8m',
		spark: [3, 5, 4, 8, 6, 11, 14],
		runsWeek: 14,
		needs: 2,
		note: '3 patterns surfacing in payments paths'
	},
	{
		id: 'ledger',
		name: 'ledger-core',
		repo: 'acme/ledger-core',
		dojoName: 'Acme Corp',
		classification: 'company',
		phase: 'adopt',
		lastRun: '2h',
		spark: [6, 7, 5, 9, 8, 9, 9],
		runsWeek: 9,
		needs: 0,
		note: 'idempotency pattern adopted org-wide'
	},
	{
		id: 'site',
		name: 'personal-site',
		repo: 'rin/personal-site',
		dojoName: null,
		classification: 'personal',
		phase: 'watch',
		lastRun: '1d',
		spark: [2, 1, 3, 2, 4, 2, 3],
		runsWeek: 3,
		needs: 0,
		note: 'no dōjō · your ladder alone'
	}
];

// A "you"-context nav: personal groups on top (Relay · you / Me), a Work group
// below — the shape the AppShell binds to.
export const nav: KitNavGroup[] = [
	{
		group: 'Relay · you',
		items: [
			{ id: 'today', kanji: '今', label: 'Today', badge: 4 },
			{ id: 'runs', icon: 'eye', label: 'Live runs' }
		]
	},
	{
		group: 'Me',
		items: [
			{ id: 'projects', icon: 'folder', label: 'Projects' },
			{ id: 'contributions', kanji: '共', label: 'Contributions' }
		]
	}
];

export const tabs: KitNavItem[] = [
	{ id: 'today', kanji: '今', label: 'Today', badge: 4 },
	{ id: 'projects', icon: 'folder', label: 'Projects' },
	{ id: 'runs', icon: 'eye', label: 'Runs' }
];

// ── Governance ─────────────────────────────────────────────────────────────

export const ladder: KitLadderRung[] = [
	{
		id: 'company',
		kanji: '社',
		scope: 'Company',
		name: 'Acme Corp',
		caption: 'your employer · every project',
		tone: 'ink',
		rules: [
			{ kanji: '守', text: 'No secrets in source — vault only, never .env in git', hard: true },
			{ kanji: '守', text: 'Never log tokens or PII, even at debug level', hard: true },
			{ kanji: '理', text: 'Public APIs stay backward-compatible two minor versions' },
			{ kanji: '検', text: 'Coverage ≥ 80% on money- or auth-touching paths', hard: true }
		]
	},
	{
		id: 'client',
		kanji: '客',
		scope: 'Client',
		name: 'Globex',
		caption: 'engagement rung · switches on for client repos',
		tone: 'accent',
		rules: [
			{ kanji: '盾', text: 'Verify webhook signatures before parsing the body', hard: true },
			{ kanji: '盾', text: 'Client identifiers never leave the machine — derived only' }
		]
	},
	{
		id: 'personal',
		kanji: '己',
		scope: 'Personal',
		name: 'Rin Saito',
		caption: 'your standing preferences · every project',
		tone: 'ink',
		rules: [
			{ kanji: '己', text: 'Explain the plan before editing more than three files' },
			{ kanji: '己', text: 'Prefer small, reviewable commits over one large diff' }
		]
	}
];

export const conflicts: KitConflict[] = [
	{
		id: 'cf1',
		topic: 'retry behaviour on money-moving calls',
		loser: { level: 'Company', text: 'retry freely on transient failure' },
		winner: { level: 'Project', text: 'idempotency key required before retry' },
		why: 'More specific scope refines the broader one — Project > Company.',
		locked: false
	},
	{
		id: 'cf2',
		topic: 'logging verbosity in auth boundary',
		loser: { level: 'Stack', text: 'debug-log request/response bodies' },
		winner: { level: 'Company', text: 'never log tokens or PII (★)' },
		why: 'A non-negotiable locks — no narrower scope can relax it.',
		locked: true
	}
];

export const stance: KitStanceDial[] = [
	{
		id: 'autonomy',
		kanji: '任',
		label: 'autonomy',
		caption: 'how far a session runs before it asks',
		levels: ['ask always', 'ask on guarded', 'ask on risky', 'run freely'],
		value: 1
	},
	{
		id: 'sharing',
		kanji: '共',
		label: 'sharing',
		caption: 'what surfaces to the dōjō',
		levels: ['private', 'patterns only', 'patterns + prompts', 'everything derived'],
		value: 1
	},
	{
		id: 'review',
		kanji: '検',
		label: 'review',
		caption: 'who signs off before a rule adopts',
		levels: ['me alone', 'one maintainer', 'two maintainers', 'quorum'],
		value: 2
	}
];

// ── Relay ──────────────────────────────────────────────────────────────────

export const runs: KitRun[] = [
	{
		id: 's-2891',
		project: 'lumen-auth',
		assistant: 'claude-sonnet',
		state: 'running',
		task: 'refactor refresh-token rotation',
		elapsed: '38m',
		edits: 12,
		gate: true
	},
	{
		id: 's-2890',
		project: 'agency-monorepo',
		assistant: 'claude-opus',
		state: 'waiting',
		task: 'wire initech billing webhook',
		elapsed: '12m',
		edits: 4,
		gate: true
	},
	{
		id: 's-2887',
		project: 'ledger-core',
		assistant: 'claude-sonnet',
		state: 'running',
		task: 'add idempotency keys to ledger writes',
		elapsed: '1h 4m',
		edits: 27,
		gate: false
	}
];

export const gates: KitGate[] = [
	{
		id: 'g1',
		project: 'lumen-auth',
		cmd: 'pnpm db:migrate --env=staging',
		kind: 'command',
		risk: 'guarded',
		why: 'touches an auth-boundary schema',
		session: 's-2891',
		age: '3m'
	},
	{
		id: 'g2',
		project: 'agency-monorepo',
		cmd: 'gh secret set STRIPE_KEY',
		kind: 'secret',
		risk: 'high',
		why: 'writes a secret — company guard requires approval',
		session: 's-2890',
		age: '1m'
	}
];

export const needsYou: KitNeed[] = [
	{
		id: 'n1',
		kind: 'gate',
		project: 'lumen-auth',
		dojo: 'Acme Corp',
		title: 'run migration against staging db',
		age: '3m',
		why: 'touches an auth-boundary schema · guarded'
	},
	{
		id: 'n2',
		kind: 'conflict',
		project: 'agency-monorepo',
		dojo: 'Initech',
		title: 'retry policy clashes with idempotency rule',
		age: '26m',
		why: 'Company ‘retry freely’ vs Team ‘idempotency key required’'
	},
	{
		id: 'n3',
		kind: 'decision',
		project: 'globex-portal',
		dojo: 'Globex',
		title: 'adopt ‘verify webhook signature’ as a client guard',
		age: '1h',
		why: '4 sessions · dereferenced · confidence 0.91'
	},
	{
		id: 'n4',
		kind: 'review',
		project: 'lumen-auth',
		dojo: 'Acme Corp',
		title: 'second approval · ‘never log refresh tokens’',
		age: '2h',
		why: 'Keiko approved · needs one more maintainer'
	}
];

export const decisions: KitDecision[] = [
	{
		id: 'd1',
		project: 'globex-portal',
		title: 'adopt ‘verify webhook signature’ as a client guard',
		options: ['adopt to Client rung', 'keep as project note', 'decline'],
		context: '4 sessions · dereferenced · confidence 0.91',
		age: '1h'
	},
	{
		id: 'd2',
		project: 'lumen-auth',
		title: 'promote ‘idempotency key’ from Project to Company',
		options: ['promote to Company', 'keep at Project', 'decline'],
		context: 'adopted in 6 repos · no conflicts',
		age: '5h'
	}
];

export const chat: KitChatTurn[] = [
	{
		who: 'sensei',
		kanji: '先',
		text: 'Noticed the refresh-token rotation touches the logger. There is a company guard against logging tokens — want me to route the debug line through the redaction sink?',
		when: '38m'
	},
	{
		who: 'rin',
		text: 'Yes. And add the integration test the project rung asks for.',
		when: '36m'
	},
	{
		who: 'sensei',
		kanji: '先',
		text: 'Drafting the test against the device-code flow. One command needs approval to run the staging migration — it is in your needs-you band.',
		when: '35m'
	},
	{ who: 'rin', text: 'Approving now.', when: '3m' }
];
