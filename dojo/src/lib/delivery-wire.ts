// A workspace's delivery facts on the wire (spec docs/spec/daemon/tracker-delivery-metrics.md
// D3, D4, D5; #352): one declaration of each shape, for the server reads, the
// client fetch and the ingest alike.
import type { SprintFigureValues } from '@sensei/sdlc-figures';

/** A sprint's SprintSdlc as the daemon computed it: the seven figures, their
 *  reasons, and the counts behind them. */
export interface DeliverySdlc extends SprintFigureValues {
	facts: Record<string, number> | null;
}

/** One sprint of a board's kept copy, as the reads answer it. */
export interface DeliverySprintWire {
	external_id: string;
	seq: number;
	name: string;
	state: 'future' | 'active' | 'closed';
	start_at: string | null;
	end_at: string | null;
	complete_at: string | null;
	sdlc: DeliverySdlc | null;
	/** The words of the reason codes in `sdlc`, by code. */
	reason_words: Record<string, string> | null;
}

/** One board of a workspace with its kept copy's facts and every sprint. */
export interface DeliveryBoardWire {
	board_id: string;
	board_key: string;
	kind: string;
	name: string;
	predictability: number | null;
	predictability_reason: string | null;
	predictability_words: string | null;
	history_at: string | null;
	pending_tickets: number;
	reported_at: string;
	sprints: DeliverySprintWire[];
}

export interface WorkspaceDeliveryWire {
	boards: DeliveryBoardWire[];
}

/** One non-removed ticket: the title is its only free text (B9). */
export interface DeliveryTicketWire {
	key: string;
	title: string | null;
	status_category: string | null;
	points: number | null;
	sprints: string[];
	issue_created_at: string | null;
}
