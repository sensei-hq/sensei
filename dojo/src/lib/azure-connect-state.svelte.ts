// Connect Azure DevOps (#308 R5b, D36/D42) — the panel's state, on an
// organization dōjō's Admin → Integrations and in the personal dōjō.
//
// Connect sends the person to Microsoft Entra (the server answers the URL). Back
// on the screen, the pending connect's organizations are listed, with a warning
// when the consent carries more than read scopes, and each can be imported —
// into this dōjō, or as its own. A dōjō's connections (when the api lists them)
// show each organization's state; sync re-reads with the stored grant. No token
// ever passes through here. A failure says why and leaves no invented list.
import { DojoApiError } from './dojo-api';
import {
	connectNotice,
	importLine,
	syncLine,
	type AzureConnectionView,
	type AzureGrantView,
	type AzureImportResult,
	type AzureStatusView,
	type AzureSyncResult
} from './azure-view';

/** The Azure calls, already bound to the dōjō (or the person) and session. The
 *  personal dōjō has no connection list (status, sync, removeToken). */
export interface AzureConnectApi {
	start: () => Promise<{ url: string }>;
	/** Send the browser to a URL (Microsoft's sign-in). */
	navigate: (url: string) => void;
	organizations: () => Promise<AzureGrantView>;
	importOrg: (organization: string) => Promise<AzureImportResult>;
	status?: () => Promise<AzureStatusView>;
	sync?: (organizationId: string) => Promise<AzureSyncResult>;
	removeToken?: (organizationId: string) => Promise<unknown>;
	/** After an import — e.g. re-read the person's dōjōs. */
	afterImport?: () => Promise<unknown>;
}

/** The server's answer when there is no pending connect. */
const NO_GRANT = 428;

const reason = (e: unknown) => (e instanceof DojoApiError ? e.message : 'could not reach the dōjō service');

export class AzureConnectState {
	connections = $state<AzureConnectionView[]>([]);
	/** Whether this server can connect; null when the screen cannot tell. */
	configured = $state<boolean | null>(null);
	/** The pending connect's organizations, or null when there is none. */
	grant = $state<AzureGrantView | null>(null);
	/** `'connect'`, `'list'`, an organization's name or id while acting on it. */
	busy = $state<string | null>(null);
	error = $state<string | null>(null);
	/** What the last act came to. */
	message = $state<string | null>(null);
	/** The last import's answer — where the organization went. */
	lastImport = $state<AzureImportResult | null>(null);

	constructor(private readonly api: AzureConnectApi) {}

	/** The callback's outcome, from the page's query string. */
	notice(q: URLSearchParams): void {
		const n = connectNotice(q);
		if (!n) return;
		this.error = n.ok ? null : n.text;
		this.message = n.ok ? n.text : null;
	}

	/** Read the dōjō's connections (when listed) and the pending connect. */
	async load(): Promise<void> {
		this.busy = 'list';
		try {
			if (this.api.status) {
				try {
					const view = await this.api.status();
					this.connections = view.connections;
					this.configured = view.configured;
				} catch (e) {
					this.connections = [];
					this.error = reason(e);
				}
			}
			try {
				this.grant = await this.api.organizations();
			} catch (e) {
				this.grant = null;
				if (!(e instanceof DojoApiError && e.status === NO_GRANT)) this.error = reason(e);
			}
		} finally {
			this.busy = null;
		}
	}

	/** Start the connect: the browser goes to Microsoft. */
	async connect(): Promise<void> {
		this.busy = 'connect';
		this.error = null;
		this.message = null;
		try {
			const { url } = await this.api.start();
			this.api.navigate(url);
		} catch (e) {
			this.error = reason(e);
		} finally {
			this.busy = null;
		}
	}

	/** Import one organization of the pending connect. */
	async importOrg(organization: string): Promise<void> {
		this.busy = organization;
		this.error = null;
		this.message = null;
		try {
			const result = await this.api.importOrg(organization);
			this.lastImport = result;
			this.message = importLine(result);
		} catch (e) {
			this.error = reason(e);
			this.busy = null;
			return;
		}
		this.busy = null;
		await this.api.afterImport?.();
		await this.load();
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
			this.message = `The connection to ${c.name} was removed`;
		});
	}

	/** Forget everything read — the screen moved to another dōjō. */
	reset(): void {
		this.connections = [];
		this.configured = null;
		this.grant = null;
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
		// Re-read either way: a refused sync may have just marked the connection
		// "reconnect", and the list should say so.
		const error = this.error;
		await this.load();
		this.error = this.error ?? error;
	}
}
