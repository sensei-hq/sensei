// Settings · Extensions — the installed skills, agents and hooks, and turning
// one on or off. The read is the loader's (+page.ts). A toggle the daemon
// refuses leaves the row as it was and says why on that row.
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { failureReason, type ApiResult } from '$lib/api.js';
import type { ExtensionRow } from '../settings-reads';

export interface ExtensionsApi {
  setInstalledItemEnabled: (name: string, kind: string, enabled: boolean) => Promise<ApiResult<void>>;
}

export type ExtensionsRead = { extensions: ExtensionRow[]; error: string | null };

const keyOf = (e: ExtensionRow) => `${e.kind}::${e.name}`;

export class ExtensionsState {
  extensions = $state<ExtensionRow[]>([]);
  /** Why the list could not be read (#288) — not "none installed". */
  loadError = $state<string | null>(null);
  #busy = new SvelteSet<string>();
  #errors = new SvelteMap<string, string>();

  constructor(read: ExtensionsRead) {
    this.setRead(read);
  }

  setRead(read: ExtensionsRead): void {
    this.extensions = read.extensions;
    this.loadError = read.error;
  }

  isBusy(e: ExtensionRow): boolean {
    return this.#busy.has(keyOf(e));
  }

  toggleError(e: ExtensionRow): string | null {
    return this.#errors.get(keyOf(e)) ?? null;
  }

  /** Busy per row, so a slow toggle does not disable its neighbours. */
  async toggle(api: ExtensionsApi, e: ExtensionRow, enabled: boolean): Promise<void> {
    const key = keyOf(e);
    this.#busy.add(key);
    this.#errors.delete(key);
    const res = await api.setInstalledItemEnabled(e.name, e.kind, enabled);
    if (res.ok) {
      this.extensions = this.extensions.map((x) => (keyOf(x) === key ? { ...x, enabled } : x));
    } else {
      this.#errors.set(key, `Not ${enabled ? 'enabled' : 'disabled'}: ${failureReason(res.error)}`);
    }
    this.#busy.delete(key);
  }
}
