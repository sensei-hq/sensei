// The workspace window's metrics, shared by the Metrics landing and every metric's
// detail: one WorkspaceMetrics per workspace, provided by the workspace layout so
// what was read survives moving to another section and back.
import { getContext, setContext } from 'svelte';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { WorkspaceMetrics, type MetricsApi } from './metrics-state.svelte.js';
import { ActionItemsState } from './action-items-state.svelte.js';

/** Show a file in Finder — the Tauri opener, loaded when first used. */
const revealInFinder = async (path: string): Promise<void> => {
  const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
  await revealItemInDir(path);
};

/** The context key, exported for a page mounted on its own in a test. */
export const METRICS_HOST = Symbol('metrics-host');

export class MetricsHost {
  #workspaceId: () => string;
  #api: () => MetricsApi;
  #current: WorkspaceMetrics | null = null;

  #reveal: (path: string) => Promise<void>;
  #actions: { for: WorkspaceMetrics; state: ActionItemsState } | null = null;

  constructor(
    workspaceId: () => string,
    api: () => MetricsApi = () => senseiApi(appState.port),
    reveal: (path: string) => Promise<void> = revealInFinder,
  ) {
    this.#workspaceId = workspaceId;
    this.#api = api;
    this.#reveal = reveal;
  }

  /** The action items' state for the workspace shown now: one per workspace
   *  metrics (so per workspace, by the id's value), acting through that
   *  workspace's URL and settling that workspace's list. */
  get actions(): ActionItemsState {
    const metrics = this.metrics;
    if (this.#actions?.for !== metrics) {
      this.#actions = {
        for: metrics,
        state: new ActionItemsState({
          api: this.#api(),
          workspaceId: () => metrics.workspaceId,
          reveal: this.#reveal,
          settle: (id) => metrics.settleRecommendation(id),
        }),
      };
    }
    return this.#actions.state;
  }

  /** The workspace's metrics. Kept by the id's VALUE: SvelteKit replaces
   *  `page.params` on every navigation, and an equal id must not stand for a new,
   *  unread set. */
  get metrics(): WorkspaceMetrics {
    const id = this.#workspaceId();
    if (this.#current?.workspaceId !== id) this.#current = new WorkspaceMetrics(this.#api(), id);
    return this.#current;
  }
}

export const provideMetricsHost = (host: MetricsHost): MetricsHost => setContext(METRICS_HOST, host);
export const metricsHost = (): MetricsHost => getContext<MetricsHost>(METRICS_HOST);
