// Connect Azure DevOps (#308 R5b, D35/D36) — the panel's state, on an
// organization dōjō's Admin → Integrations and in the personal dōjō.
//
// The admin types the organization URL and pastes a personal access token. The
// token is sent ONCE, to the server, which validates it before storing it in
// Vault; this state empties the field after every request, success or failure,
// and never keeps, shows or sends it anywhere else. A dōjō's connections (when
// the api can list them) show each organization's token state; sync re-reads
// with the stored token. A failure says why and leaves no invented list.
import { DojoApiError } from './dojo-api';
import { importLine, syncLine, type AzureConnectionView, type AzureImportResult, type AzureStatusView, type AzureSyncResult } from './azure-view';

/** The Azure calls, already bound to the dōjō (or the person) and session. Only
 *  `connect` is needed: the personal dōjō has no connection list. */
export interface AzureConnectApi {
	status?: () => Promise<AzureStatusView>;
	connect: (organizationUrl: string, token: string) => Promise<AzureImportResult>;
	sync?: (organizationId: string) => Promise<AzureSyncResult>;
	removeToken?: (organizationId: string) => Promise<unknown>;
}

const reason = (e: unknown) => (e instanceof DojoApiError ? e.message : 'could not reach the dōjō service');

export class AzureConnectState {
	organizationUrl = $state('');
	/** The pasted PAT — emptied after every request. */
	token = $state('');
	connections = $state<AzureConnectionView[]>([]);
	/** Whether a status read has succeeded and `connections` is its answer. */
	loaded = $state(false);
	/** `'connect'`, `'list'`, or an organization's id while acting on it. */
	busy = $state<string | null>(null);
	error = $state<string | null>(null);
	/** What the last act came to. */
	message = $state<string | null>(null);
	/** The last import's answer — where the organization went. */
	lastImport = $state<AzureImportResult | null>(null);

	constructor(private readonly api: AzureConnectApi) {}

	/** Read the dōjō's Azure organizations (when the api lists them). */
	async load(): Promise<void> {
		if (!this.api.status) return;
		this.busy = 'list';
		try {
			const view = await this.api.status();
			this.connections = view.connections;
			this.loaded = true;
		} catch (e) {
			this.connections = [];
			this.loaded = false;
			this.error = reason(e);
		} finally {
			this.busy = null;
		}
	}

	/** Send the URL and token once; the field is emptied whatever the answer. */
	async connect(): Promise<void> {
		const url = this.organizationUrl.trim();
		const token = this.token.trim();
		this.error = null;
		this.message = null;
		if (!url) {
			this.error = 'Enter the organization URL (https://dev.azure.com/your-organization)';
			return;
		}
		if (!token) {
			this.error = 'Paste a personal access token';
			return;
		}
		this.busy = 'connect';
		try {
			const result = await this.api.connect(url, token);
			this.lastImport = result;
			this.message = importLine(result);
		} catch (e) {
			this.error = reason(e);
		} finally {
			this.token = '';
			this.busy = null;
		}
		if (this.message) await this.load();
	}

	sync(c: AzureConnectionView): Promise<void> {
		const sync = this.api.sync;
		if (!sync) return Promise.resolve();
		return this.#act(c, async () => {
			this.message = syncLine(await sync(c.id));
		});
	}

	removeToken(c: AzureConnectionView): Promise<void> {
		const remove = this.api.removeToken;
		if (!remove) return Promise.resolve();
		return this.#act(c, async () => {
			await remove(c.id);
			this.message = `The token for ${c.name} was removed`;
		});
	}

	/** Forget everything read — the screen moved to another dōjō. */
	reset(): void {
		this.organizationUrl = '';
		this.token = '';
		this.connections = [];
		this.loaded = false;
		this.busy = null;
		this.error = null;
		this.message = null;
		this.lastImport = null;
	}

	async #act(c: AzureConnectionView, run: () => Promise<void>): Promise<void> {
		this.busy = c.id;
		this.error = null;
		this.message = null;
		try {
			await run();
		} catch (e) {
			this.error = reason(e);
		}
		this.busy = null;
		// Re-read either way: a refused sync may have just marked the token
		// "needs a new token", and the list should say so.
		await this.load();
	}
}
