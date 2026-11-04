// Settings · Extensions — the installed skills, agents and hooks, and turning
// one on or off. The read is the loader's (+page.ts); Retry re-reads through
// the same read (never invalidateAll, which re-runs every load on the route).
// A toggle the daemon refuses leaves the row as it was and says why on that row.
import { failureNotice } from '@sensei/ui';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { failureReason, type ApiResult } from '$lib/api.js';
import { readExtensions, type ExtensionRow, type SkippedExtension } from '../settings-reads';

export interface ExtensionsApi {
  setInstalledItemEnabled: (name: string, kind: string, enabled: boolean) => Promise<ApiResult<void>>;
}

export type ExtensionsRead = {
  extensions: ExtensionRow[];
  skipped?: SkippedExtension[];
  error: string | null;
};

const keyOf = (e: ExtensionRow) => `${e.kind}::${e.name}`;

export class ExtensionsState {
  extensions = $state<ExtensionRow[]>([]);
  /** Items the daemon sent that cannot be listed, each with its reason (D83).
   *  Absent when every row was listed — the read's own shape, not a filled-in
   *  empty list. */
  skipped = $state<SkippedExtension[] | undefined>(undefined);
  /** Why the list could not be read (#288) — not "none installed". */
  loadError = $state<string | null>(null);
  /** The read is in flight (Retry), so it cannot be pressed twice. */
  reading = $state(false);
  #busy = new SvelteSet<string>();
  #errors = new SvelteMap<string, string>();

  constructor(read: ExtensionsRead) {
    this.setRead(read);
  }

  setRead(read: ExtensionsRead): void {
    this.extensions = read.extensions;
    this.skipped = read.skipped;
    this.loadError = read.error;
    this.#errors.clear();
  }

  /** Re-read the list through the page's own read. A failed re-read stays the
   *  error — the empty list is only ever a read that said there is nothing. */
  async load(api: { tryGetInstalledItems: Parameters<typeof readExtensions>[0]['tryGetInstalledItems'] }): Promise<void> {
    this.reading = true;
    try {
      this.setRead(await readExtensions(api));
    } finally {
      this.reading = false;
    }
  }

  isBusy(e: ExtensionRow): boolean {
    return this.#busy.has(keyOf(e));
  }

  toggleError(e: ExtensionRow): string | null {
    return this.#errors.get(keyOf(e)) ?? null;
  }

  /** Notice props for a toggle that did not land (packages/ui/notice.js). The
   *  wording follows the click's intent, so an unknown row reads "on". */
  toggleNotice(e: ExtensionRow) {
    return failureNotice(`Couldn't switch ${e.name} ${e.enabled === true ? 'off' : 'on'}`, this.toggleError(e));
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
