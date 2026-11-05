// A workspace's delivery facts shaped for the dōjō page (spec
// tracker-delivery-metrics D5; #352): each board's predictability and each sprint's
// seven figures, worded as the desktop app words them (@sensei/sdlc-figures) — a
// value in its unit, or its reason's words, never a bare code; sprints in the order
// read (the daemon's).
import { points, sdlcFigure, sprintSdlcFigures, type SdlcFigure } from '@sensei/sdlc-figures';
import type { DeliveryBoardWire, DeliverySprintWire, WorkspaceDeliveryWire } from './workspace-delivery-data';

export interface DeliverySprintView {
	id: string;
	name: string;
	state: DeliverySprintWire['state'];
	dates: string;
	figures: SdlcFigure[];
}

export interface DeliveryBoardView {
	id: string;
	name: string;
	predictability: SdlcFigure;
	pendingTickets: number;
	reportedAt: string;
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
		figures: sprintSdlcFigures(s.sdlc ?? NO_FIGURES, words(s.reason_words))
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
		sprints: b.sprints.map(sprintView)
	};
}

export function toDeliveryView(wire: WorkspaceDeliveryWire): { boards: DeliveryBoardView[] } {
	return { boards: wire.boards.map(boardView) };
}
