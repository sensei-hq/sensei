// Test fixtures for the dojo kit specs — lifted from the mockup mock data
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
	KitRulePack,
	KitContribution,
	KitDownstream,
	KitContribStat,
	KitRun,
	KitGate,
	KitNeed,
	KitDecision,
	KitChatTurn,
	KitConstitutionSection,
	KitTriageGroup,
	KitCandidateDetail,
	KitApproval,
	KitKnowledge,
	KitEngagement,
	KitConfidentiality,
	KitIncident,
	KitClientAuditRow,
	KitMember,
	KitRolePolicy,
	KitScopeOwner,
	KitBilling,
	KitIdentity,
	KitHealth
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
	},
	{
		id: 'stack',
		kanji: '技',
		scope: 'Stack',
		name: 'React · TypeScript',
		caption: 'most specific · refines everything above',
		tone: 'ink',
		rules: [
			{ kanji: '技', text: 'No default exports in shared packages' },
			{ kanji: '技', text: 'Server state through the query layer, never in a store' }
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

// Adoptable rule-pack bundles (dojo2-data `rulePacks`) — NOT a "library". A pack
// is a credible, recognizable governance bundle a dōjō would adopt into its
// constitution, spanning the governance areas (core principles · architecture ·
// security · compliance · language/stack · design system). Sourced to real
// standards + house authors, not "by ACME"; a couple ship already-adopted.
export const rulePacks: KitRulePack[] = [
	{
		id: 'clean-code',
		kanji: '理',
		name: 'Clean Code · core principles',
		by: 'R. Martin · Clean Code',
		adopted: true,
		note: 'small functions, intention-revealing names, single responsibility',
		rules: [
			'A function does one thing and does it at a single level of abstraction',
			'Keep functions small — a screenful is already too long',
			'Name things to reveal intent — no encodings, no cryptic abbreviations',
			'Prefer no more than three arguments; pass an object past that',
			'A function has no side effects its name does not promise',
			'Return early — avoid deep nesting and long if/else ladders',
			'Comment the why, never restate the what the code already says',
			'A class or module has a single reason to change',
			'Depend on abstractions, not on concrete implementations',
			'No duplicated logic — extract the third near-identical copy',
			'Handle errors with exceptions, not returned error codes',
			'Never pass or return null across a boundary you control',
			'Leave each file cleaner than you found it',
			'Tests are first-class code — kept readable and kept passing'
		]
	},
	{
		id: 'owasp-asvs',
		kanji: '守',
		name: 'OWASP ASVS · application security',
		by: 'OWASP · ASVS 4.0',
		adopted: true,
		note: 'input validation, auth boundary, secrets & session handling',
		rules: [
			'Validate every input against an allow-list on the server',
			'Encode output for the context it renders into to stop injection',
			'Use parameterized queries — never concatenate untrusted input',
			'Enforce authentication on every non-public request',
			'Check authorization on every access, not just at the menu',
			'Store passwords with a slow, salted hash (argon2 or bcrypt)',
			'Rotate the session id on login and on privilege change',
			'Set session cookies HttpOnly, Secure, and SameSite',
			'Lock, throttle, or MFA-gate repeated failed logins',
			'Keep secrets out of source — load them from a vault at runtime',
			'Never log credentials, tokens, or session identifiers',
			'Verify TLS and pin trust for outbound service calls',
			'Deny by default — fail closed when a check cannot complete',
			'Regenerate and expire tokens on logout',
			'Enforce a strong content-security-policy on every response',
			'Protect state-changing requests against CSRF',
			'Cap request and upload size to blunt resource exhaustion',
			'Scan and pin third-party dependencies for known CVEs',
			'Return generic errors — never leak stack traces to the client',
			'Sign and verify webhooks before parsing the body',
			'Segregate secrets and data by environment — no prod keys in dev',
			'Log security events with enough context to investigate'
		]
	},
	{
		id: 'pci-dss',
		kanji: '盾',
		name: 'PCI-DSS · cardholder data',
		by: 'PCI-DSS v4.0',
		adopted: false,
		note: 'no PAN in logs, encrypted at rest, scoped access & audit trail',
		rules: [
			'Never store the full PAN in logs, URLs, or analytics',
			'Mask the PAN to first-six/last-four wherever it is displayed',
			'Never store sensitive authentication data after authorization',
			'Encrypt stored cardholder data with strong, rotated keys',
			'Encrypt cardholder data in transit over public networks',
			'Keep encryption keys separate from the data they protect',
			'Restrict data access to a documented need-to-know',
			'Give each person a unique id — no shared accounts',
			'Require MFA for all access into the cardholder data environment',
			'Log every access to cardholder data with user and timestamp',
			'Retain audit trails for at least one year, tamper-evident',
			'Scope the environment — segment card systems off the flat network'
		]
	},
	{
		id: 'twelve-factor',
		kanji: '築',
		name: 'Twelve-Factor · service architecture',
		by: 'Heroku · 12-Factor',
		adopted: false,
		note: 'config in env, stateless processes, dev/prod parity',
		rules: [
			'Track one codebase per app in version control, deployed many places',
			'Declare and isolate dependencies explicitly — never rely on the host',
			'Store config in the environment, never in the code',
			'Treat backing services as attachable resources swapped by config',
			'Strictly separate the build, release, and run stages',
			'Run the app as one or more stateless, share-nothing processes',
			'Export services via a port binding — be self-contained',
			'Scale out by the process model, not by threads in one box',
			'Start fast and shut down gracefully on SIGTERM',
			'Keep dev, staging, and prod as similar as possible',
			'Treat logs as an event stream written to stdout',
			'Run admin tasks as one-off processes in the same environment'
		]
	},
	{
		id: 'rust-clippy',
		kanji: '技',
		name: 'Rust · clippy defaults',
		by: 'Rust · clippy',
		adopted: false,
		note: 'clippy-clean, Result over unwrap, no blocking in async',
		rules: [
			'cargo clippy passes with no warnings before merge',
			'Prefer Result and ? over unwrap or expect in non-test code',
			'Never block a thread inside an async function',
			'Borrow with &str and &[T] in signatures over owned String and Vec',
			'Derive Debug, Clone, and PartialEq rather than hand-writing them',
			'Model absence with Option, not a sentinel value',
			'Avoid unnecessary clone — let the borrow checker guide ownership',
			'Gate every unsafe block behind a comment justifying its soundness',
			'Prefer iterator chains over manual index loops'
		]
	},
	{
		id: 'ts-strict',
		kanji: '技',
		name: 'TypeScript · strict',
		by: 'TypeScript · strict',
		adopted: true,
		note: 'no any, exhaustive switches, no non-null assertions',
		rules: [
			'strict mode is on — no implicit any anywhere',
			'Never use the any type — reach for unknown and narrow',
			'Avoid the non-null assertion — handle the null case',
			'Make switches exhaustive with a never check in the default',
			'Type function boundaries explicitly — no inferred public any',
			'Prefer readonly and const assertions for data that never changes',
			'Model unions as discriminated unions, not loose flags',
			'No unchecked index access — guard array and record lookups'
		]
	},
	{
		id: 'svelte-runes',
		kanji: '技',
		name: 'Svelte 5 · runes',
		by: 'Svelte 5 · runes',
		adopted: false,
		note: '$state/$derived over stores, snippets over slots, no legacy reactivity',
		rules: [
			'Use $state for reactive local state, not a writable store',
			'Derive with $derived rather than a reactive $: statement',
			'Run side effects in $effect, keeping them free of derived logic',
			'Declare component inputs with $props, not export let',
			'Compose with snippets and {@render}, not legacy slots',
			'Bind handlers with onclick, not on:click',
			'Keep shared reactive state in a .svelte.ts module'
		]
	},
	{
		id: 'zen-sumi',
		kanji: '紋',
		name: 'Zen-Sumi · design tokens',
		by: 'Zen-Sumi · design tokens',
		adopted: false,
		note: 'named tokens not hex, 8-stop scale, 4px grid, md: responsive',
		rules: [
			'Use named tokens — never a hex, oklch, or var() literal in a component',
			'Size type from the 8-stop scale — never a style font-size',
			'Space and pad on the 4px grid — never a literal px value',
			'Take radii from the token scale, not ad-hoc corner values',
			'Colour from the semantic ramp — paper, ink, accent, never raw',
			'Lay out mobile-first and add md: prefixes for desktop',
			'Keep responsive layout in utilities, not a @media in style',
			'Reach for a kit component before hand-rolling a one-off',
			'Keep a per-surface config parity so tokens resolve the same',
			'Ship dark mode from the dual palette, not overridden colours'
		]
	},
	{
		id: 'a11y-wcag',
		kanji: '観',
		name: 'WCAG 2.2 AA · accessibility',
		by: 'W3C · WCAG 2.2',
		adopted: false,
		note: 'labels & roles, focus order, contrast, keyboard-reachable',
		rules: [
			'Give every control an accessible name and the right role',
			'Make every interaction reachable and operable by keyboard alone',
			'Keep a visible, unambiguous focus indicator on every control',
			'Order focus to follow the reading and interaction flow',
			'Meet 4.5:1 text contrast — 3:1 for large text and UI edges',
			'Provide a text alternative for every meaningful image',
			'Never signal state by colour alone — pair it with text or shape',
			'Associate every form field with a persistent visible label',
			'Announce errors in text and tie them to the field',
			'Respect prefers-reduced-motion — no essential motion-only content',
			'Structure the page with landmarks and a logical heading order'
		]
	},
	{
		id: 'conventional',
		kanji: '整',
		name: 'Conventional Commits · history',
		by: 'Conventional Commits 1.0',
		adopted: false,
		note: 'typed subjects, scoped changes, small single-purpose commits',
		rules: [
			'Prefix each commit with a type (feat/fix/docs/…)',
			'Use a scope in parentheses for the affected area',
			'Breaking changes carry a `!` or a `BREAKING CHANGE:` footer',
			'Subject in imperative mood, ≤72 chars',
			'Body explains the why, not the what'
		]
	}
];

// ── Contributions ────────────────────────────────────────────────────────────

// What you shared upstream (per-destination status) + approved-for-you
// downstream + the lifetime stat row (dojo2-data `contributions`).
export const contributionsMine: KitContribution[] = [
	{
		kanji: '紋',
		title: 'Adapter wraps a third-party SDK behind a trait',
		dest: 'Acme Corp',
		scope: 'Stack · Rust',
		status: 'approved',
		when: '2d',
		note: 'published · +7pp first-try rate'
	},
	{
		kanji: '直',
		title: 'Prefer $state(...) over let in Svelte 5 components',
		dest: 'Rust Guild',
		scope: 'Stack · Svelte',
		status: 'pending',
		when: '6h',
		note: 'in triage · owner Sven K.'
	},
	{
		kanji: '盾',
		title: 'Verify webhook signature before parsing the body',
		dest: 'Globex',
		scope: 'Client · anonymized',
		status: 'approved',
		when: '1d',
		note: 'anonymized · shared safely',
		anonymous: true
	},
	{
		kanji: '問',
		title: 'Persona: integration-test author for auth flows',
		dest: 'Acme Corp',
		scope: 'Stack · React',
		status: 'declined',
		when: '3d',
		note: 'merged into an existing persona'
	}
];

export const contributionsDownstream: KitDownstream[] = [
	{
		kanji: '守',
		title: 'Never log refresh tokens, even at debug level',
		from: 'Acme Corp',
		scope: 'Company',
		when: '8m',
		adopted: false,
		kind: 'guard'
	},
	{
		kanji: '紋',
		title: 'Idempotency key on money-moving mutations',
		from: 'Acme Corp',
		scope: 'Team · Payments',
		when: '4h',
		adopted: true,
		kind: 'pattern'
	},
	{
		kanji: '技',
		title: 'Skill: explain a slow query plan',
		from: 'Rust Guild',
		scope: 'Stack · Postgres',
		when: '1d',
		adopted: false,
		kind: 'skill'
	}
];

export const contributionsStat: KitContribStat = { approved: 2, pending: 1, helped: 612 };

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

// ── Org context ──────────────────────────────────────────────────────────────

/** A project in a dōjō's jurisdiction, before the org-list mapping (dojo2-data
 *  `orgProjects[slug][]`). The team + maintainers + weekly-run cadence are the
 *  raw org-scoped fields the mockup folds into a `KitProject`. */
interface OrgProjectSeed {
	id: string;
	kanji: string;
	name: string;
	team: string;
	classification: string;
	phase: string;
	maintainers: number;
	runsWeek: number;
	needs: number;
}

// Projects under a dōjō's jurisdiction, keyed by slug (dojo2-data `orgProjects`).
// Only `acme` is authored; an unknown slug returns empty (no fake fallback) so a
// real org route renders honestly until its own `/v1` data is wired.
const orgProjectSeeds: Record<string, OrgProjectSeed[]> = {
	acme: [
		{
			id: 'auth',
			kanji: '件',
			name: 'lumen-auth',
			team: 'Payments',
			classification: 'company',
			phase: 'notice',
			maintainers: 3,
			runsWeek: 14,
			needs: 2
		},
		{
			id: 'ledger',
			kanji: '件',
			name: 'ledger-core',
			team: 'Payments',
			classification: 'company',
			phase: 'adopt',
			maintainers: 2,
			runsWeek: 9,
			needs: 0
		},
		{
			id: 'gw',
			kanji: '件',
			name: 'api-gateway',
			team: 'Platform',
			classification: 'company',
			phase: 'watch',
			maintainers: 4,
			runsWeek: 5,
			needs: 1
		},
		{
			id: 'web',
			kanji: '件',
			name: 'acme-web',
			team: 'Web',
			classification: 'company',
			phase: 'notice',
			maintainers: 2,
			runsWeek: 8,
			needs: 1
		}
	]
};

/** The projects in a dōjō's jurisdiction as `KitProject` rows (mockup mapping:
 *  `repo = slug/name`, `note = team · N maintainers`, `lastRun = N/wk`). An
 *  unknown slug returns an empty list. */
export function orgProjectsFor(slug: string): KitProject[] {
	const seeds = orgProjectSeeds[slug] ?? [];
	return seeds.map((p) => ({
		id: p.id,
		name: p.name,
		repo: slug + '/' + p.name,
		classification: p.classification,
		phase: p.phase,
		lastRun: p.runsWeek + '/wk',
		runsWeek: p.runsWeek,
		needs: p.needs,
		dojoName: null,
		note: p.team + ' · ' + p.maintainers + ' maintainers'
	}));
}

// The dōjō's OWN authored constitution, by section (dojo2-data
// `orgConstitution`). A dōjō authors rules at the scopes it owns: company-wide,
// per team, per stack (stacks also adopt rule packs). This is NOT the resolution
// ladder — that only appears at project-preview time. Keyed by slug; only `acme`
// is authored, and an unknown slug returns empty (no fake fallback).
const orgConstitutions: Record<string, KitConstitutionSection[]> = {
	acme: [
		{
			id: 'company',
			kanji: '社',
			scope: 'Company-wide',
			group: 'Company',
			caption: 'every project in the dōjō',
			rules: [
				{ kanji: '守', text: 'No secrets in source — vault only, never .env in git', hard: true },
				{ kanji: '守', text: 'Never log tokens or PII, even at debug level', hard: true },
				{ kanji: '理', text: 'Public APIs stay backward-compatible two minor versions' },
				{ kanji: '検', text: 'Coverage ≥ 80% on money- or auth-touching paths', hard: true }
			]
		},
		{
			id: 'team-pay',
			kanji: '組',
			scope: 'Payments',
			group: 'Teams',
			caption: 'payments · ledger repos',
			rules: [
				{
					kanji: '紋',
					text: 'Every money-moving mutation carries an idempotency key',
					hard: true
				},
				{ kanji: '検', text: 'Reconciliation job runs before any ledger migration' }
			]
		},
		{
			id: 'team-plat',
			kanji: '組',
			scope: 'Platform',
			group: 'Teams',
			caption: 'platform · API · gateway',
			rules: [
				{ kanji: '理', text: 'Every public endpoint carries a deprecation policy' },
				{ kanji: '守', text: 'Rate-limit and auth-check at the gateway, not the service' }
			]
		},
		{
			id: 'stack-react',
			kanji: '技',
			scope: 'React · TypeScript',
			group: 'Stacks',
			caption: 'adopted packs + rules',
			packs: ['React · TypeScript baseline'],
			rules: [
				{ kanji: '技', text: 'No default exports in shared packages' },
				{ kanji: '技', text: 'Server state through the query layer, never in a store' }
			]
		},
		{
			id: 'stack-pg',
			kanji: '技',
			scope: 'Postgres',
			group: 'Stacks',
			caption: 'no pack adopted yet',
			packs: [],
			rules: [
				{ kanji: '技', text: 'Every migration is reversible or ships a documented backout' }
			]
		}
	]
};

/** The dōjō's authored constitution sections for a slug (dojo2-data
 *  `orgConstitution[slug]`), returns empty for an unknown slug. */
export function orgConstitutionFor(slug: string): KitConstitutionSection[] {
	return orgConstitutions[slug] ?? [];
}

// ── Maintainer + lead consoles (dojo2-data `consoles`) ──────────────────────
//
// The role-console fixtures the maintainer (Govern) and lead (Clients) screens
// bind to. Ported 1:1 from the mockup `consoles` block. As with the org
// Overview data above, each console is keyed by slug; only `acme` is authored,
// and an unknown slug returns empty (no fake fallback, matching the single
// authored fixture), so a real org route renders without a per-org seed until
// the `/v1` wiring lands.

// 1 · Triage — candidate learnings awaiting a maintainer decision, grouped by
// scope, ranked by confidence within each group.
const triageGroups: Record<string, KitTriageGroup[]> = {
	acme: [
		{
			scope: 'Payments',
			items: [
				{
					id: 't1',
					kanji: '紋',
					title: 'Idempotency key on every money-moving mutation',
					origin: '6 sessions · 3 repos',
					conf: 0.91,
					conflicts: 1,
					dups: 0,
					impact: 'high'
				},
				{
					id: 't2',
					kanji: '検',
					title: 'Reconcile before any ledger migration',
					origin: 's-2887 · ledger-core',
					conf: 0.78,
					conflicts: 0,
					dups: 2,
					impact: 'normal'
				}
			]
		},
		{
			scope: 'React · TypeScript',
			items: [
				{
					id: 't3',
					kanji: '技',
					title: 'Server state through the query layer, never a store',
					origin: '11 sessions',
					conf: 0.86,
					conflicts: 0,
					dups: 1,
					impact: 'normal'
				},
				{
					id: 't4',
					kanji: '直',
					title: 'Prefer $state(...) over let in Svelte 5',
					origin: 'Rust Guild mirror',
					conf: 0.64,
					conflicts: 0,
					dups: 0,
					impact: 'low'
				}
			]
		},
		{
			scope: 'Auth boundary',
			items: [
				{
					id: 't5',
					kanji: '守',
					title: 'Never log refresh tokens, even at debug level',
					origin: 's-2891 · lumen-auth',
					conf: 0.95,
					conflicts: 0,
					dups: 0,
					impact: 'high'
				}
			]
		}
	]
};

// The detail for the selected candidate (mockup `candidateDetail`).
const candidateDetail: KitCandidateDetail = {
	learning: 'Every money-moving mutation must carry an idempotency key before retry.',
	cause: 'Two sessions retried a charge on a transient 500 and double-posted to the ledger.',
	context: 'Surfaced in payments-service across lumen-auth, ledger-core and globex-portal.',
	evidence: [
		's-2887 · double-post caught in reconciliation',
		's-2871 · manual rollback, 40 min',
		'3 more sessions'
	],
	conflict: {
		loser: 'Company · retry freely on transient failure',
		winner: 'Project · idempotency key required'
	},
	scopes: ['Company', 'Team · Payments', 'Stack · Node']
};

/** The scope-grouped triage candidates for a slug (dojo2-data
 *  `consoles.triage`), returns empty for an unknown slug. */
export function triageGroupsFor(slug: string): KitTriageGroup[] {
	return triageGroups[slug] ?? [];
}

/** The selected-candidate detail (dojo2-data `consoles.candidateDetail`). One
 *  representative detail this chunk — a real per-candidate lookup lands with the
 *  `/v1` wiring. */
export function candidateDetailFor(_slug: string): KitCandidateDetail {
	return candidateDetail;
}

// 2 · Approvals — second-approval queue for high-impact/safety candidates.
const approvals: Record<string, KitApproval[]> = {
	acme: [
		{
			id: 'a1',
			kanji: '守',
			title: 'Never log refresh tokens, even at debug level',
			scope: 'Company',
			first: 'Keiko Tanaka',
			when: '2h',
			impact: 'safety'
		},
		{
			id: 'a2',
			kanji: '紋',
			title: 'Promote idempotency key from Project to Company',
			scope: 'Company',
			first: 'Marco Diaz',
			when: '5h',
			impact: 'high'
		}
	]
};

/** The second-approval queue for a slug (dojo2-data `consoles.approvals`),
 *  returns empty for an unknown slug. */
export function approvalsFor(slug: string): KitApproval[] {
	return approvals[slug] ?? [];
}

// 3 · Knowledge — published library + prune policy + extension catalog.
const knowledge: Record<string, KitKnowledge> = {
	acme: {
		prunePolicy: 'Prune after 90 days unused',
		active: [
			{
				kanji: '紋',
				title: 'Idempotency key on money-moving mutations',
				scope: 'Team · Payments',
				adopted: '6 repos',
				age: 'adopted 3mo'
			},
			{
				kanji: '守',
				title: 'Verify webhook signature before parsing',
				scope: 'Client guard',
				adopted: '3 repos',
				age: 'adopted 1mo'
			},
			{
				kanji: '技',
				title: 'No default exports in shared packages',
				scope: 'Stack · React',
				adopted: '9 repos',
				age: 'adopted 5mo'
			}
		],
		pending: [
			{
				kanji: '理',
				title: 'Deprecation window of two minor versions',
				scope: 'Company',
				age: 'unused 84d'
			}
		],
		catalog: [
			{ kanji: '問', title: 'integration-test author', kind: 'agent', scope: 'Stack · React' },
			{ kanji: '令', title: 'explain a slow query plan', kind: 'command', scope: 'Stack · Postgres' },
			{ kanji: '技', title: 'auth-boundary reviewer', kind: 'skill', scope: 'Company' }
		]
	}
};

/** The published-knowledge library for a slug (dojo2-data `consoles.knowledge`),
 *  returns empty for an unknown slug. */
export function knowledgeFor(slug: string): KitKnowledge {
	return knowledge[slug] ?? { prunePolicy: '', active: [], pending: [], catalog: [] };
}

// 4 · Engagements — the client register + the confidentiality model.
const engagements: Record<string, KitEngagement[]> = {
	acme: [
		{
			id: 'e1',
			kanji: '客',
			client: 'Globex',
			clientTenantId: null,
			projects: 'globex-portal · billing',
			lessons: 86,
			dropped: 214,
			since: '7mo',
			status: 'active'
		},
		{
			id: 'e2',
			kanji: '客',
			client: 'Initech',
			clientTenantId: null,
			projects: 'agency-monorepo',
			lessons: 41,
			dropped: 97,
			since: '1y',
			status: 'active'
		}
	]
};

const confidentiality: KitConfidentiality = {
	kept: ['The lesson — a pattern, a guard, a skill', 'Anonymized code shape', 'Confidence & impact'],
	dropped: ['Client & repo identifiers', 'Endpoints, hostnames, secrets', 'Literal source & data'],
	example: {
		raw: 'await stripe.charges.create({ idempotencyKey })',
		stripped: 'await <payment-sdk>.<mutation>({ idempotencyKey })'
	}
};

/** The client engagements for a slug (dojo2-data `consoles.engagements`),
 *  returns empty for an unknown slug. */
export function engagementsFor(slug: string): KitEngagement[] {
	return engagements[slug] ?? [];
}

/** The confidentiality model (dojo2-data `consoles.confidentiality`) — the
 *  kept-vs-dropped boundary + the anonymized example. */
export function confidentialityFor(_slug: string): KitConfidentiality {
	return confidentiality;
}

// 5 · Incidents — confidentiality containment.
const incidents: Record<string, KitIncident[]> = {
	acme: [
		{
			id: 'i1',
			kanji: '盾',
			title: 'Near-leak: client hostname in a shared prompt',
			client: 'Globex',
			state: 'contained',
			when: '3d',
			severity: 'high'
		},
		{
			id: 'i2',
			kanji: '盾',
			title: 'Raw stack trace queued to Collective',
			client: 'Initech',
			state: 'resolved',
			when: '2w',
			severity: 'medium'
		}
	]
};

/** The confidentiality incidents for a slug (dojo2-data `consoles.incidents`),
 *  returns empty for an unknown slug. */
export function incidentsFor(slug: string): KitIncident[] {
	return incidents[slug] ?? [];
}

// 6 · Client audit — the immutable confidentiality ledger.
const clientAudit: Record<string, KitClientAuditRow[]> = {
	acme: [
		{
			t: '10:42',
			kanji: '共',
			event: 'Lesson shared upstream',
			detail: 'idempotency pattern · anonymized',
			client: 'Globex',
			ok: true
		},
		{
			t: '10:41',
			kanji: '盾',
			event: 'Stripped 2 identifiers',
			detail: 'hostname, repo slug',
			client: 'Globex',
			ok: true
		},
		{
			t: '09:18',
			kanji: '却',
			event: 'Blocked contribution',
			detail: 'raw source detected · held',
			client: 'Initech',
			ok: false
		},
		{
			t: 'Yesterday',
			kanji: '共',
			event: 'Lesson shared upstream',
			detail: 'webhook guard · anonymized',
			client: 'Globex',
			ok: true
		}
	]
};

/** The immutable client-audit ledger for a slug (dojo2-data
 *  `consoles.clientAudit`), returns empty for an unknown slug. */
export function clientAuditFor(slug: string): KitClientAuditRow[] {
	return clientAudit[slug] ?? [];
}

// ── Admin consoles (dojo2-data `members` · `roles` · `scopeOwners` ·
//    `billing` · `consoles.identity` · `consoles.health`) ────────────────────
//
// The admin-console fixtures the Members & Roles / Policies / Audit surface, the
// Scopes screen, the Identity & SSO screen, the Health / Monitor screen and the
// Plan & billing screen bind to. Ported 1:1 from the mockup. The org-scoped
// consoles (scope owners) are keyed by slug; only `acme` is authored and an
// unknown slug returns empty (no fake fallback, matching the single authored
// fixture); the org-wide fixtures (members, roles, billing, identity, health)
// are single authored fixtures the accessor returns for any org this chunk. A
// real per-org lookup lands with the `/v1` wiring.

// Members & Roles — git-derived role + dōjō overrides (dojo2-data `members`).
export const members: KitMember[] = [
	{ name: 'Keiko Tanaka', git: 'Org owner', role: 'admin', scopes: 'all', active: 'now' },
	{
		name: 'Marco Diaz',
		git: 'Repo admin',
		role: 'maintainer',
		scopes: 'Payments · Ledger',
		active: '12m'
	},
	{
		name: 'Rin Saito',
		git: 'Repo admin',
		role: 'maintainer',
		scopes: 'Web · Auth',
		active: 'now',
		you: true
	},
	{
		name: 'Sven Karlsson',
		git: 'Repo admin',
		role: 'maintainer',
		scopes: 'Platform · API',
		active: '3h'
	},
	{ name: 'Aiko Nakamura', git: 'Write', role: 'developer', scopes: 'Web · Auth', active: '1h' },
	{ name: 'Tom Becker', git: 'Read', role: 'developer', scopes: '—', active: '5d' }
];

/** The org's members (dojo2-data `members`). One authored fixture this chunk. */
export function membersFor(_slug: string): KitMember[] {
	return members;
}

// Role policies — additive rungs, each adds capability (dojo2-data `roles`).
export const rolePolicies: KitRolePolicy[] = [
	{ id: 'developer', kanji: '士', label: 'developer', note: 'read-mostly · watches & chats' },
	{ id: 'maintainer', kanji: '掟', label: 'maintainer', note: 'authors governance · triages' },
	{ id: 'lead', kanji: '客', label: 'lead', note: 'client engagements & audit' },
	{ id: 'admin', kanji: '任', label: 'admin', note: 'member roles & policies' }
];

/** The org's role policies (dojo2-data `roles`). Additive: developer →
 *  maintainer → lead → admin. */
export function rolePoliciesFor(_slug: string): KitRolePolicy[] {
	return rolePolicies;
}

/** The Audit-tab log — a derived read of the shared `chat` thread (mockup
 *  ScrRoleSurfaces audit tab maps `D2.chat.thread`). Reuses the existing `chat`
 *  fixture rather than duplicating a near-identical list. */
export function auditLogFor(_slug: string): KitChatTurn[] {
	return chat;
}

// Scope ownership — who owns/triages each scope's queue (dojo2-data `scopeOwners`).
const scopeOwners: Record<string, KitScopeOwner[]> = {
	acme: [
		{
			scope: 'Company-wide',
			group: 'Company',
			owner: 'Keiko Tanaka',
			role: 'admin',
			queue: 3,
			sla: '24h'
		},
		{ scope: 'Payments', group: 'Teams', owner: 'Marco Diaz', role: 'maintainer', queue: 5, sla: '12h' },
		{
			scope: 'Platform',
			group: 'Teams',
			owner: 'Sven Karlsson',
			role: 'maintainer',
			queue: 2,
			sla: '24h'
		},
		{
			scope: 'React · TypeScript',
			group: 'Stacks',
			owner: 'Rin Saito',
			role: 'maintainer',
			queue: 4,
			sla: '48h'
		},
		{ scope: 'Postgres', group: 'Stacks', owner: null, role: null, queue: 1, sla: 'fallback' }
	]
};

/** The scope-ownership rows for a slug (dojo2-data `scopeOwners`); returns empty
 *  for an unknown slug. */
export function scopeOwnersFor(slug: string): KitScopeOwner[] {
	return scopeOwners[slug] ?? [];
}

// Plan & billing — the business model (dojo2-data `billing`).
export const billing: KitBilling = {
	plan: 'Team · private',
	perSeat: 12,
	seatsActive: 34,
	seatsReadonly: 14,
	renews: 'Aug 1',
	tiers: [
		{
			id: 'free',
			kanji: '無',
			name: 'Free',
			price: 'Free',
			sub: 'public · OSS · personal',
			lines: [
				'Public / open-source or personal solo dōjō',
				'Unlimited members · full governance authoring',
				'Relay for your own projects — watch · approve · decide · chat',
				'Fair use: 1 active machine · standard realtime'
			]
		},
		{
			id: 'team',
			kanji: '組',
			name: 'Team',
			price: 'Per seat',
			sub: '/ mo · active contributor',
			current: true,
			lines: [
				'Private, shared scopes for a company or team',
				'Role consoles · client engagements · audit',
				'Relay across the team — shared inbox, presence, priority realtime',
				'Read-only members always free'
			]
		},
		{
			id: 'ent',
			kanji: '企',
			name: 'Enterprise',
			price: 'Contract',
			sub: 'custom',
			dark: true,
			lines: [
				'Self-hosted / VPC · SSO (OIDC / SAML) + SCIM',
				'Audit retention & export · air-gapped bundle',
				'Self-hosted relay · SSO on mobile',
				'SLA & priority support'
			]
		}
	],
	relayRows: [
		{ label: 'Relay on your own projects — watch · approve · decide · chat', free: true },
		{ label: 'One active machine · standard realtime · native push', free: true },
		{ label: "Shared team inbox & queue · presence (who's handling this)", free: false },
		{ label: 'Higher concurrency · priority realtime · approval audit trail', free: false }
	],
	invoices: [
		{ d: 'Jul 1, 2026', amt: '$408.00', s: 'paid' },
		{ d: 'Jun 1, 2026', amt: '$396.00', s: 'paid' },
		{ d: 'May 1, 2026', amt: '$372.00', s: 'paid' }
	]
};

/** The org's plan & billing state (dojo2-data `billing`). One authored fixture
 *  this chunk. */
export function billingFor(_slug: string): KitBilling {
	return billing;
}

// Identity & SSO — IdP + SCIM + git/device-code mappings (dojo2-data
// `consoles.identity`).
export const identity: KitIdentity = {
	idp: { name: 'Okta', protocol: 'OIDC', status: 'connected', domain: 'acme.okta.com' },
	scim: true,
	mappings: [
		{ source: 'GitHub org · acme', to: 'auto-join · role from repo access', count: 41 },
		{ source: 'Magic link · @acme.com', to: 'developer by default', count: 5 },
		{ source: 'Device code', to: 'read-only', count: 2 }
	]
};

/** The org's identity & SSO config (dojo2-data `consoles.identity`). One
 *  authored fixture this chunk. */
export function identityFor(_slug: string): KitIdentity {
	return identity;
}

// Health / Monitor — the shared mind's vital signs (dojo2-data
// `consoles.health`).
export const health: KitHealth = {
	signals: [
		{ kanji: '観', label: 'Sessions this week', n: '312', sub: '↑ 14%', tone: 'accent' },
		{ kanji: '覚', label: 'Adoption rate', n: '68%', sub: 'of approved', tone: 'success' },
		{ kanji: '盾', label: 'Leak-guard blocks', n: '3', sub: 'all contained', tone: 'warning' },
		{ kanji: '門', label: 'Queue age · median', n: '6h', sub: 'within SLA', tone: 'ink' }
	],
	contribVsApprove: [
		{ wk: 'W1', c: 18, a: 12 },
		{ wk: 'W2', c: 22, a: 15 },
		{ wk: 'W3', c: 19, a: 17 },
		{ wk: 'W4', c: 26, a: 20 }
	],
	alerts: [
		{
			kanji: '盾',
			title: 'Leak-guard held a raw stack trace',
			detail: 'Initech · auto-contained · no data left',
			when: '2h',
			sev: 'resolved'
		},
		{
			kanji: '門',
			title: 'Postgres scope queue has no owner',
			detail: '1 candidate routed to fallback',
			when: '1d',
			sev: 'warning'
		}
	]
};

/** The org's health / monitor vitals (dojo2-data `consoles.health`). One
 *  authored fixture this chunk. */
export function healthFor(_slug: string): KitHealth {
	return health;
}
