// Admin → Integrations (#308 R5a) — the screen's state. The person pastes a
// forge token; it stays in this object's memory for the screen's life and goes
// only into request bodies (never a URL, never storage). Every act re-reads the
// list, because what the person may do next depends on what the act changed.
// A failure says why and leaves no stale or invented list on screen.
import { DojoApiError } from './dojo-api';
import type { IntegrationUnit, IntegrationUnitsView, MapTarget } from './integrations-view';

/** The Integrations calls, already bound to the dōjō, provider and session. */
export interface IntegrationsApi {
	list: (token: string) => Promise<IntegrationUnitsView>;
	claim: (token: string, unitId: string) => Promise<unknown>;
	map: (token: string, unitId: string, target: MapTarget) => Promise<unknown>;
}

const message = (e: unknown) =>
	e instanceof DojoApiError ? e.message : 'could not reach the dōjō service';

export class IntegrationsState {
	/** The pasted forge token. */
	token = $state('');
	units = $state<IntegrationUnit[]>([]);
	/** Whether the last read saw every unit. */
	complete = $state(true);
	/** Whether a read has succeeded and `units` is its answer. */
	loaded = $state(false);
	/** `'list'` while reading; a unit's id while acting on it. */
	busy = $state<string | null>(null);
	error = $state<string | null>(null);
	/** The workspace name typed per unit id. */
	names = $state<Record<string, string>>({});

	constructor(private readonly api: IntegrationsApi) {}

	/** Read the units the token sees. */
	async load(): Promise<void> {
		const token = this.token.trim();
		if (!token) {
			this.error = 'Paste a GitHub token first.';
			return;
		}
		this.busy = 'list';
		this.error = null;
		try {
			const view = await this.api.list(token);
			this.units = view.units;
			this.complete = view.complete;
			this.loaded = true;
		} catch (e) {
			this.units = [];
			this.loaded = false;
			this.error = message(e);
		} finally {
			this.busy = null;
		}
	}

	/** Forget the token and everything read with it — the screen moved to another
	 *  dōjō, whose acts would otherwise run against this one's listing. */
	reset(): void {
		this.token = '';
		this.units = [];
		this.complete = true;
		this.loaded = false;
		this.busy = null;
		this.error = null;
		this.names = {};
	}

	claim(unit: IntegrationUnit): Promise<void> {
		return this.#act(unit, (token) => this.api.claim(token, unit.id));
	}

	mapAsDojo(unit: IntegrationUnit): Promise<void> {
		return this.#act(unit, (token) => this.api.map(token, unit.id, { as: 'dojo' }));
	}

	mapAsWorkspace(unit: IntegrationUnit): Promise<void> {
		const name = (this.names[unit.id] ?? '').trim();
		const target: MapTarget = name ? { as: 'workspace', name } : { as: 'workspace' };
		return this.#act(unit, (token) => this.api.map(token, unit.id, target));
	}

	/** Run one act, then re-read; say why on failure and keep the list. */
	async #act(unit: IntegrationUnit, write: (token: string) => Promise<unknown>): Promise<void> {
		this.busy = unit.id;
		this.error = null;
		try {
			await write(this.token.trim());
		} catch (e) {
			this.error = message(e);
			this.busy = null;
			return;
		}
		this.busy = null;
		await this.load();
	}
}
