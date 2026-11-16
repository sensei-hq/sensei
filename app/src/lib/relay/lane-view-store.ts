// Where Relay keeps the person's Lanes view (#422): this browser's storage, a per-viewer
// convenience, read and written as the workspaces view is (`browserStorage` already
// answers null where there is no usable store, and the view then starts by run).
import type { RelayLaneView } from '@sensei/state';
import { STORAGE_KEYS, browserStorage } from '$lib/storage-keys.js';

export function relayLaneViewStore(): { read(): string | null; write(view: RelayLaneView): void } {
  return {
    read: () => browserStorage()?.getItem(STORAGE_KEYS.relayLaneView) ?? null,
    write: (view) => browserStorage()?.setItem(STORAGE_KEYS.relayLaneView, view),
  };
}
