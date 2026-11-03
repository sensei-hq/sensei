// The Code map the diagrams layout shares with its lens pages: one state per
// workspace, so switching lens in the sidebar keeps the level, the selection and
// what was already read.
import { getContext, setContext } from 'svelte';
import { CodeMapState, budgetLine, drawnBands, workspaceStats, type CodeMapPort, type Stat } from '@sensei/codemap';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { codemapPort } from './codemap-port.js';

/** The context key, exported for a page mounted on its own in a test. */
export const CODEMAP_HOST = Symbol('codemap-host');

export class CodeMapHost {
  readonly port: CodeMapPort;
  #workspaceId: () => string;
  #current: { id: string; state: CodeMapState } | null = null;

  /** The workspace's map. Kept by the id's VALUE: SvelteKit replaces `page.params`
   *  on every navigation, and an equal id must not stand for a new, unread map. */
  get state(): CodeMapState {
    const id = this.#workspaceId();
    if (this.#current?.id !== id) this.#current = { id, state: new CodeMapState() };
    return this.#current.state;
  }

  /** The workspace's numbers for the header; null until the map is read. */
  get stats(): Stat[] | null {
    const l0 = this.state.l0;
    return l0 ? workspaceStats(l0) : null;
  }

  /** What the drawing shows of what there is, under the current filters. */
  get budget(): string | null {
    const m = this.state;
    if (!m.l0) return null;
    return budgetLine(m.l0, drawnBands(m.lens, m.l0, m.cochange, { edgeMode: m.edgeMode, minWeight: m.minWeight }).length);
  }

  /** Where the map is, for the header's level tag. */
  get level(): string {
    return `L${this.state.crumbs.length - 1} · ${this.state.level}`;
  }

  constructor(workspaceId: () => string, port?: CodeMapPort) {
    this.#workspaceId = workspaceId;
    this.port = port ?? codemapPort(senseiApi(appState.port), workspaceId);
  }
}

export const provideCodeMapHost = (host: CodeMapHost): CodeMapHost => setContext(CODEMAP_HOST, host);
export const codeMapHost = (): CodeMapHost => getContext<CodeMapHost>(CODEMAP_HOST);
