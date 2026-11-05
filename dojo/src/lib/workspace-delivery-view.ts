// A workspace's delivery facts and team effort shaped for the dōjō page (spec
// tracker-delivery-metrics D5, B16; #352, #353): each board's predictability and each sprint's
// seven figures, worded as the desktop app words them (@sensei/sdlc-figures) — a
// value in its unit, or its reason's words, never a bare code; sprints in the order
// read (the daemon's).
import {
	activeMinutes,
	apiCostText,
	compactDuration,
	compactTokens,
	pct,
	points,
	sdlcFigure,
	sprintSdlcFigures,
	type SdlcFigure
} from '@sensei/sdlc-figures';
import type { DeliveryTeamFields } from './delivery-wire';
import type { DeliveryBoardWire, DeliverySprintWire, WorkspaceDeliveryWire } from './workspace-delivery-data';

/** An item's team effort as shown: who contributed and the merged figures (B16:
 *  none from one contributor). */
export interface TeamView {
	/** "from 3 contributors · last report …", "one contributor", or null when nobody reported. */
	line: string | null;
	figures: SdlcFigure[];
}

/** A team figure: its key, its label, and how a value is worded (given the item's values). */
type TeamFigure = readonly [string, string, (x: number, values: Record<string, unknown>) => string];

const minutes = (m: number) => compactDuration(Math.round(m));
const count = (n: number) => String(n);

/** A sprint's team effort (D5): the team's AI time and API cost in the sprint. */
export const SPRINT_TEAM: readonly TeamFigure[] = [
	['active_secs', 'team AI time', (x) => compactDuration(activeMinutes(x))],
	['api_cost', 'team API cost', (x, v) => apiCostText(x, Number(v.api_cost_partial) > 0)]
];

/** The board's team effort per ticket (D1 project row). */
export const PROJECT_TEAM: readonly TeamFigure[] = [
	['tokens_per_ticket', 'tokens per ticket', (x) => compactTokens(x)],
	['prompts_per_ticket', 'prompts per ticket', (x) => (Math.round(x * 10) / 10).toString()]
];

/** A ticket's merged figures (D1); never a cost at a contributor's rate (B16). */
export const TICKET_TEAM: readonly TeamFigure[] = [
	['worked_minutes', 'worked (person-time)', minutes],
	['prompts', 'prompts', count],
	['ai_token_cost', 'AI cost', (x, v) => apiCostText(x, Number(v.api_cost_partial) > 0)],
	['cost_per_prompt_cycle', 'cost per prompt', (x) => apiCostText(x, false)],
	['total_defects', 'defects', count],
	['testing_efficiency', 'caught by tests', pct],
	['build_failure_rate', 'build failures', pct],
	['mean_build_recovery_minutes', 'build recovery', minutes],
	['on_hold_ratio', 'on hold', pct],
	['post_review_rework_ratio', 'rework after review', pct],
	['execution_hours', 'execution', (h) => minutes(h * 60)],
	['lead_hours', 'lead time', (h) => minutes(h * 60)]
];

/** How a team figure is read (D5): summed person-time, and its known limits. */
export const TEAM_EXPLAINER =
	'Team figures add up each contributor’s own sessions, so time is person-time. Each person’s work is read on its own: a defect that escapes, a build that recovers, a submission or rework that spans two people’s sessions counts for each person apart, so these can differ from one view of everyone’s sessions. An item with one contributor, and cost at each contributor’s own rate, are not shown here.';

const dayTime = (iso: string) =>
	new Date(iso).toLocaleString('en', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', hourCycle: 'h23', timeZone: 'UTC' }) + ' UTC';

/** An item's team effort, worded: a partial figure ends in "+", a null shows its
 *  reason's words, never 0. */
export function teamView(t: DeliveryTeamFields, defs: readonly TeamFigure[]): TeamView {
	const line =
		t.contributors === 0
			? null
			: t.contributors === 1 || !t.team
				? 'one contributor'
				: `from ${t.contributors} contributors` + (t.last_report ? ` · last report ${dayTime(t.last_report)}` : '');
	if (!t.team || t.contributors < 2) return { line, figures: [] };
	const team = t.team;
	const figures = defs.map(([key, label, format]) => {
		const raw = team.values[key];
		const value = typeof raw === 'number' ? raw : null;
		const f = sdlcFigure(key, label, value, (x) => format(x, team.values), team.reasons, words(t.team_words));
		return value !== null && (team.partial[key] ?? 0) > 0 && !f.text.endsWith('+') ? { ...f, text: `${f.text}+` } : f;
	});
	return { line, figures };
}

export interface DeliverySprintView {
	id: string;
	name: string;
	state: DeliverySprintWire['state'];
	dates: string;
	figures: SdlcFigure[];
	team: TeamView;
}

export interface DeliveryBoardView {
	id: string;
	name: string;
	predictability: SdlcFigure;
	pendingTickets: number;
	reportedAt: string;
	team: TeamView;
	sprints: DeliverySprintView[];
}

const NO_FIGURES = {
	commitment_reliability: null,
	total_completion: null,
	scope_change: null,
	carry_over_rate: null,
	unplanned_work_ratio: null,
	velocity: null,
	points_commitment_reliability: null,
	reasons: {}
};

/** `{ code: { summary } }` from the wire's `{ code: summary }`. */
const words = (byCode: Record<string, string> | null) =>
	Object.fromEntries(Object.entries(byCode ?? {}).map(([code, summary]) => [code, { summary }]));

const day = (iso: string | null) =>
	iso ? new Date(iso).toLocaleDateString('en', { month: 'short', day: 'numeric', timeZone: 'UTC' }) : null;

function dates(s: DeliverySprintWire): string {
	const from = day(s.start_at);
	const to = day(s.complete_at ?? s.end_at);
	return from && to ? `${from} – ${to}` : (from ?? to ?? 'No dates');
}

function sprintView(s: DeliverySprintWire): DeliverySprintView {
	return {
		id: s.external_id,
		name: s.name,
		state: s.state,
		dates: dates(s),
		figures: sprintSdlcFigures(s.sdlc ?? NO_FIGURES, words(s.reason_words)),
		team: teamView(s, SPRINT_TEAM)
	};
}

function boardView(b: DeliveryBoardWire): DeliveryBoardView {
	const reasons: Record<string, string> = b.predictability_reason ? { predictability: b.predictability_reason } : {};
	const predictabilityWords = b.predictability_reason && b.predictability_words ? { [b.predictability_reason]: b.predictability_words } : {};
	return {
		id: b.board_id,
		name: b.name,
		predictability: sdlcFigure('predictability', 'predictability', b.predictability, points, reasons, words(predictabilityWords)),
		pendingTickets: b.pending_tickets,
		reportedAt: b.reported_at,
		team: teamView(b, PROJECT_TEAM),
		sprints: b.sprints.map(sprintView)
	};
}

export function toDeliveryView(wire: WorkspaceDeliveryWire): { boards: DeliveryBoardView[] } {
	return { boards: wire.boards.map(boardView) };
}
