// A workspace's delivery facts and team effort on the wire (spec
// docs/spec/daemon/tracker-delivery-metrics.md D3, D4, D5; #352, #353): one declaration of each shape, for the server reads, the
// client fetch and the ingest alike.
import type { SprintFigureValues } from '@sensei/sdlc-figures';

/** A sprint's SprintSdlc as the daemon computed it: the seven figures, their
 *  reasons, and the counts behind them. */
export interface DeliverySdlc extends SprintFigureValues {
	facts: Record<string, number> | null;
}

/** One item's effort merged across its contributors (D1, A3): every null has its
 *  reason; a figure some contributor did not know has its count in `partial`. */
export interface DeliveryTeamWire {
	values: Record<string, unknown>;
	reasons: Record<string, string>;
	partial: Record<string, number>;
}

/** One contributor's own figures on an item, for a reader allowed them (A4). */
export interface DeliveryPersonWire {
	principal_id: string;
	display_name: string | null;
	team: DeliveryTeamWire;
	reported_at: string;
	/** Their last report is more than 7 days old. */
	stale: boolean;
}

/** An item's team effort as this reader sees it (B16, A4). A reader allowed every
 *  contributor's own numbers gets the full merge and `people`; anyone else gets
 *  `team` null from fewer than two contributors, and `last_report` too. */
export interface DeliveryTeamFields {
	team: DeliveryTeamWire | null;
	/** The words of the reason codes in `team`, by code. */
	team_words: Record<string, string> | null;
	contributors: number;
	last_report: string | null;
	/** Each contributor's own figures; null for a reader not allowed them. */
	people: DeliveryPersonWire[] | null;
	/** Some contributor's last report is more than 7 days old. */
	stale: boolean;
}

/** One sprint of a board's kept copy, as the reads answer it. */
export interface DeliverySprintWire extends DeliveryTeamFields {
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
export interface DeliveryBoardWire extends DeliveryTeamFields {
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
export interface DeliveryTicketWire extends DeliveryTeamFields {
	key: string;
	title: string | null;
	status_category: string | null;
	points: number | null;
	sprints: string[];
	issue_created_at: string | null;
}
