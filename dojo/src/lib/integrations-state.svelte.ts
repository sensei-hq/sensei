// Admin → Integrations (#308 R5a) — the screen's state. It reads the orgs the
// person's CONNECTED GitHub sees; the token never passes through here — the
// server holds it in a sealed cookie set when "Connect GitHub" re-authorized.
// No connection (the server's 428) is not an error: the screen offers Connect
// GitHub. Every act re-reads the list, because what the person may do next
// depends on what it changed. A failure says why and leaves no stale or
// invented list on screen.
import { DojoApiError } from './dojo-api';
import type { IntegrationUnit, IntegrationUnitsView, MapTarget } from './integrations-view';

/** The Integrations calls, already bound to the dōjō, provider and session. */
export interface IntegrationsApi {
	list: () => Promise<IntegrationUnitsView>;
	claim: (unitId: string) => Promise<unknown>;
	map: (unitId: string, target: MapTarget) => Promise<unknown>;
	/** Re-authorize with GitHub through the dōjō's sign-in, back to this screen. */
	connect: () => Promise<unknown>;
}

/** The server's answer when there is no GitHub connection to read with. */
const NOT_CONNECTED = 428;

const message = (e: unknown) =>
	e instanceof DojoApiError ? e.message : 'could not reach the dōjō service';

export class IntegrationsState {
	units = $state<IntegrationUnit[]>([]);
	/** Whether the last read saw every unit. */
	complete = $state(true);
	/** Whether a read has succeeded and `units` is its answer. */
	loaded = $state(false);
	/** No GitHub connection (or one GitHub stopped honouring): offer Connect GitHub. */
	needsConnect = $state(false);
	/** `'list'` while reading; `'connect'` while re-authorizing; a unit's id while acting on it. */
	busy = $state<string | null>(null);
	error = $state<string | null>(null);
	/** The workspace name typed per unit id. */
	names = $state<Record<string, string>>({});

	constructor(private readonly api: IntegrationsApi) {}

	/** Read the units the connected GitHub sees. */
	async load(): Promise<void> {
		this.busy = 'list';
		this.error = null;
		try {
			const view = await this.api.list();
			this.units = view.units;
			this.complete = view.complete;
			this.loaded = true;
			this.needsConnect = false;
		} catch (e) {
			this.#lost(e);
		} finally {
			this.busy = null;
		}
	}

	/** Start the GitHub re-authorization; the page reloads the list when it returns. */
	async connect(): Promise<void> {
		this.busy = 'connect';
		this.error = null;
		try {
			await this.api.connect();
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'could not start the GitHub connection';
		} finally {
			this.busy = null;
		}
	}

	/** Forget everything read — the screen moved to another dōjō. */
	reset(): void {
		this.units = [];
		this.complete = true;
		this.loaded = false;
		this.needsConnect = false;
		this.busy = null;
		this.error = null;
		this.names = {};
	}

	claim(unit: IntegrationUnit): Promise<void> {
		return this.#act(unit, () => this.api.claim(unit.id));
	}

	mapAsDojo(unit: IntegrationUnit): Promise<void> {
		return this.#act(unit, () => this.api.map(unit.id, { as: 'dojo' }));
	}

	mapAsWorkspace(unit: IntegrationUnit): Promise<void> {
		const name = (this.names[unit.id] ?? '').trim();
		const target: MapTarget = name ? { as: 'workspace', name } : { as: 'workspace' };
		return this.#act(unit, () => this.api.map(unit.id, target));
	}

	/** A read failed: no list on screen; Connect GitHub when the connection is the
	 *  reason, the reason otherwise. */
	#lost(e: unknown): void {
		this.units = [];
		this.loaded = false;
		if (e instanceof DojoApiError && e.status === NOT_CONNECTED) {
			this.needsConnect = true;
		} else {
			this.error = message(e);
		}
	}

	/** Run one act, then re-read; say why on failure and keep the list — unless the
	 *  connection lapsed, which only Connect GitHub mends. */
	async #act(unit: IntegrationUnit, write: () => Promise<unknown>): Promise<void> {
		this.busy = unit.id;
		this.error = null;
		try {
			await write();
		} catch (e) {
			if (e instanceof DojoApiError && e.status === NOT_CONNECTED) this.#lost(e);
			else this.error = message(e);
			this.busy = null;
			return;
		}
		this.busy = null;
		await this.load();
	}
}
