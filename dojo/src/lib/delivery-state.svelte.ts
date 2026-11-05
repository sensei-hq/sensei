// The dōjō Delivery page's selection (spec tracker-delivery-metrics D5; #352): one
// sprint selected per board, and its tickets read on demand. A failed read is an
// error, never an empty list; a slower earlier answer is dropped.
import type { DeliveryTicketWire } from './workspace-delivery-data';
import { TICKET_TEAM, teamView, type TeamView } from './workspace-delivery-view';

/** One ticket as the page lists it: its key, its title when the tracker gave one,
 *  and the team's merged effort on it (B16: none from one contributor). */
export interface DeliveryTicketView {
	key: string;
	title: string | null;
	team: TeamView;
}

export type ReadTickets = (boardId: string, sprintId: string) => Promise<DeliveryTicketWire[]>;

export class DeliveryPageState {
	#read: ReadTickets;
	/** Bumped per selection, so a slower earlier answer is dropped. */
	#seq = 0;

	selected = $state<{ board: string; sprint: string } | null>(null);
	tickets = $state<DeliveryTicketView[] | null>(null);
	ticketsError = $state<string | null>(null);

	constructor(read: ReadTickets) {
		this.#read = read;
	}

	/** The sprint selected on this board, if any. */
	selectedFor(boardId: string): string | null {
		return this.selected?.board === boardId ? this.selected.sprint : null;
	}

	async select(boardId: string, sprintId: string) {
		const seq = ++this.#seq;
		this.selected = { board: boardId, sprint: sprintId };
		this.tickets = null;
		this.ticketsError = null;
		try {
			const rows = await this.#read(boardId, sprintId);
			if (seq !== this.#seq) return;
			this.tickets = rows.map((t) => ({ key: t.key, title: t.title, team: teamView(t, TICKET_TEAM) }));
		} catch (e) {
			if (seq !== this.#seq) return;
			this.ticketsError = e instanceof Error ? e.message : 'the request failed';
		}
	}
}
