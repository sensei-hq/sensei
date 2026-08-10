/*
 * Sensei product site — editorial "surfaces" content.
 *
 * Ported from docs/mockups/Sensei/site/surfaces.jsx (HeroBrief + the five
 * SurfaceBlocks). This is the single source of truth for the copy + mechanic
 * data that the #gallery section and the hero card render — deliberately NOT
 * app screenshots, which go stale the moment the UI moves. A surface's GOAL
 * and FLOW stay true release to release, so we describe those instead.
 *
 * Rendering lives in routes/sensei/+page.svelte (small snippets), styled with
 * UnoCSS utilities + named tokens. `tone` values map to text-<token> classes
 * and the accent border colours in the page.
 */

/** Semantic colour keys → text-<token> / var(--<token>) in the page. */
export type Tone = 'accent' | 'success' | 'warning' | 'ink-soft' | 'ink-mute';

/** A lead paragraph, split into segments so a phrase can be emphasised (ink). */
export interface LeadSegment {
	t: string;
	em?: boolean;
}

// ── Mechanic payloads (one per surface motif) ───────────────────────
export interface AnatomyRow {
	kanji: string;
	label: string;
	priority: 'P0' | 'P1' | 'P2';
	desc: string;
}
export interface Lane {
	kanji: string;
	title: string;
	tone: Tone;
	desc: string;
}
export interface Chip {
	kanji: string;
	title: string;
	desc: string;
}
export interface FlowStep {
	title: string;
	who: string;
	desc: string;
}
export interface AnatomyCard {
	title: string;
	desc: string;
}
export interface Pill {
	kanji: string;
	label: string;
}
export interface Mode {
	kanji: string;
	title: string;
	desc: string;
}

// ── Mechanic variants (discriminated on `kind`) ─────────────────────
export interface AnatomyRowsMech {
	kind: 'anatomy-rows';
	label: string;
	rows: AnatomyRow[];
}
export interface LanesMech {
	kind: 'lanes';
	label: string;
	lanes: Lane[];
}
export interface ChipsMech {
	kind: 'chips';
	label: string;
	chips: Chip[];
}
export interface FlowMech {
	kind: 'flow';
	label: string;
	steps: FlowStep[];
}
export interface CardsMech {
	kind: 'cards';
	label: string;
	cards: AnatomyCard[];
}
export interface PillsMech {
	kind: 'pills';
	label: string;
	pills: Pill[];
	note: string;
}
export interface ModesMech {
	kind: 'modes';
	label: string;
	modes: Mode[];
}

export type Mechanic =
	| AnatomyRowsMech
	| LanesMech
	| ChipsMech
	| FlowMech
	| CardsMech
	| PillsMech
	| ModesMech;

/** One of the five surfaces: identity → headline → lead → why → mechanics. */
export interface Surface {
	n: string;
	kanji: string;
	name: string;
	role: string;
	headline: string;
	lead: LeadSegment[];
	why: string;
	mechanics: Mechanic[];
}

// ── Hero centrepiece — the product's voice, not a screenshot ────────
export interface HeroSecondary {
	kanji: string;
	label: string;
	text: string;
	tag: string;
	tone: Tone;
}
export interface HeroBrief {
	meta: string;
	metaRight: string;
	focalKanji: string;
	eyebrow: string;
	headline: string;
	leadBefore: string;
	leadCode: string;
	leadAfter: string;
	projected: string;
	provenance: string;
	secondaryLabel: string;
	secondary: HeroSecondary[];
}

export const heroBrief: HeroBrief = {
	meta: 'sensei speaks · 09:12',
	metaRight: 'an example morning',
	focalKanji: '聴',
	eyebrow: 'The one thing worth noticing',
	headline: 'The AI does not know your auth.',
	leadBefore: 'Three sessions corrected this week in ',
	leadCode: 'lumen-auth',
	leadAfter:
		" — all touched refresh or device flow. There's no integration-test persona for this module yet.",
	projected: 'Projected First-Try-Right +14% in Lumen Cloud',
	provenance: 'from s-2891 · s-2889 · s-2886',
	secondaryLabel: 'And quietly, today — kept out of the way',
	secondary: [
		{
			kanji: '繰',
			label: 'Pattern recurring',
			text: 'cache invalidation missed again',
			tag: '3rd time',
			tone: 'warning'
		},
		{
			kanji: '昇',
			label: 'Teaching adopted',
			text: 'canvas smoothing promoted to a rule',
			tag: '+7% FTR',
			tone: 'success'
		},
		{
			kanji: '探',
			label: 'Drift detected',
			text: 'brand-tokens README is 47 days old',
			tag: 'low',
			tone: 'ink-mute'
		}
	]
};

export const surfaces: Surface[] = [
	{
		n: '01',
		kanji: '観',
		name: 'Today',
		role: '家 · the morning brief',
		headline: 'One thing worth your attention. The rest stays silent.',
		lead: [
			{ t: 'Out of everything sensei watched overnight, Today elevates a ' },
			{ t: 'single', em: true },
			{
				t: ' focal observation and keeps the rest out of sight. Most mornings it is nearly empty — that emptiness is earned, and it is the point.'
			}
		],
		why: 'A dashboard of twelve widgets teaches you to ignore all twelve. One ranked item forces a decision and respects a morning.',
		mechanics: [
			{
				kind: 'anatomy-rows',
				label: 'What it surfaces · in priority order',
				rows: [
					{
						kanji: '聴',
						label: 'The focal teaching',
						priority: 'P0',
						desc: 'The single most important pattern right now — phrased as a koan, with its projected impact and the sessions it came from.'
					},
					{
						kanji: '繰',
						label: 'Also worth noticing',
						priority: 'P1',
						desc: 'At most three secondary signals: a pattern recurring, a teaching adopted, a doc quietly drifting.'
					},
					{
						kanji: '昇',
						label: 'System has learned',
						priority: 'P2',
						desc: 'Rules sensei has already adopted on your behalf — each with its scope and the session it came from.'
					},
					{
						kanji: '診',
						label: 'First-Try-Right · 14d',
						priority: 'P2',
						desc: 'The one number that says whether you and your assistant are getting better together.'
					}
				]
			}
		]
	},
	{
		n: '02',
		kanji: '録',
		name: 'Sessions',
		role: '録 · the week in review',
		headline: 'The week, read back to you in three honest lanes.',
		lead: [
			{
				t: "Every session sensei witnessed, digested into a retrospective you didn't have to run. Not a log to scroll — a verdict, in plain language, with the trend underneath."
			}
		],
		why: "Charts you have to decode get ignored. Three lanes and a single trend line don't.",
		mechanics: [
			{
				kind: 'lanes',
				label: 'The retro · three lanes',
				lanes: [
					{
						kanji: '良',
						title: 'Going well',
						tone: 'success',
						desc: 'Compound refactors, consistent naming, clean test boundaries — the habits worth keeping.'
					},
					{
						kanji: '破',
						title: 'Not going well',
						tone: 'accent',
						desc: "Tests skipped, useEffect overreach, PRs that sit too long — friction caught while it's small."
					},
					{
						kanji: '観',
						title: 'Insights',
						tone: 'ink-soft',
						desc: "New idioms forming, recurring shapes in error handling — things sensei noticed but won't judge yet."
					}
				]
			},
			{
				kind: 'chips',
				label: 'How a session is read',
				chips: [
					{ kanji: '録', title: 'Captured', desc: 'every turn, edit, test and correction' },
					{
						kanji: '診',
						title: 'Scored',
						desc: 'first-try-right, or corrected — and how many times'
					},
					{
						kanji: '印',
						title: 'Checkpointed',
						desc: 'marks where an adopted rule changed the trajectory'
					}
				]
			}
		]
	},
	{
		n: '03',
		kanji: '今',
		name: 'Insights',
		role: '今 · candidates for triage',
		headline: 'Candidate patterns, scored — you decide which become memory.',
		lead: [
			{
				t: "The patterns sensei is tracking but hasn't adopted: each with a confidence, the number of projects it's appeared in, and how long it's been forming. Sensei proposes; you dispose."
			}
		],
		why: 'A system that adopts on its own becomes a black box you have to fight. Keeping the promotion in your hands keeps the trust intact.',
		mechanics: [
			{
				kind: 'flow',
				label: 'The flow · noticed to adopted',
				steps: [
					{
						title: 'Noticed',
						who: 'Sensei',
						desc: 'A shape repeats across sessions and gets logged as a candidate.'
					},
					{
						title: 'Scored',
						who: 'Sensei',
						desc: 'Confidence rises with corroboration; spread and recency are tracked.'
					},
					{
						title: 'Reviewed',
						who: 'You',
						desc: 'Surfaced here with its evidence. You read the provenance, not a verdict.'
					},
					{
						title: 'Promoted',
						who: 'You',
						desc: 'A nod turns it into a memory — or you dismiss it and it stops asking.'
					}
				]
			}
		]
	},
	{
		n: '04',
		kanji: '覚',
		name: 'Memories',
		role: '覚 · adopted teachings',
		headline: 'Named, dated, and traceable to the sessions they came from.',
		lead: [
			{
				t: 'An adopted memory is a small, named lesson sensei applies to future matching sessions — with your blessing. Every one is auditable down to the moments that formed it.'
			}
		],
		why: "A learning system you can't inspect is a liability. Every memory shows its receipts, and every adoption is reversible.",
		mechanics: [
			{
				kind: 'cards',
				label: 'Anatomy of a memory',
				cards: [
					{
						title: 'When to apply',
						desc: 'The precise condition the lesson holds under — and when to lift it instead.'
					},
					{
						title: 'Examples watched',
						desc: 'The real sessions it was drawn from, dated and linked. Receipts, not assertions.'
					},
					{
						title: 'Provenance',
						desc: 'Seen across N sessions · first observed · confidence · adopted by whom.'
					}
				]
			},
			{
				kind: 'pills',
				label: 'Always your call',
				pills: [
					{ kanji: '採', label: 'Adopt' },
					{ kanji: '磨', label: 'Refine' },
					{ kanji: '捨', label: 'Dismiss' }
				],
				note: 'adopt, refine, or dismiss — nothing is permanent.'
			}
		]
	},
	{
		n: '05',
		kanji: '具',
		name: 'Instruments',
		role: '具 · your tools, observed',
		headline: 'Your tools, observed — tried in isolation, replayed in full.',
		lead: [
			{
				t: 'An assistant is only as good as the instruments it can reach. Instruments lets you exercise each MCP tool in isolation and replay exactly what was called during a session.'
			}
		],
		why: "When a session goes wrong, the tool is as likely a culprit as the model. You can't trust what you can't watch.",
		mechanics: [
			{
				kind: 'modes',
				label: 'Two ways to look · one toolset',
				modes: [
					{
						kanji: '具',
						title: 'Playground',
						desc: 'Run any MCP tool in isolation, with real arguments, and read the raw result. Sensei watches each call to learn which tools earn their place.'
					},
					{
						kanji: '録',
						title: 'Replay',
						desc: 'Step back through exactly what the assistant called during a session — the order, the inputs, the latency — so a surprise becomes legible.'
					}
				]
			}
		]
	}
];
