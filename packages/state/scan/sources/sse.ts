// The daemon's scan source (#496): the scan client for reads and triggers,
// the scan socket (`/api/scan/events`) for the stream. A broadcast has no
// replay, so every open is reported — the first and each reconnect — and
// the state reads on it: nothing before the socket opened is missed.
import type { ScanApi } from '../client.js';
import type { ScanSource } from '../scan-state.svelte.js';
import type { StreamFrame } from '../../events.js';
import { sseEvents, type EventStream } from '../../source.js';

/** What the source needs of a socket: the EventManager's shape, carrying scan frames. */
export type ScanEventStream = EventStream<StreamFrame>;

/** A ScanSource over the daemon: `api` for reads and triggers, `stream()` for a fresh socket per subscription. */
export function scanSseSource(api: ScanApi, stream: () => ScanEventStream): ScanSource {
  return {
    readWorkspaces: () => api.tryListWorkspaces(),
    readRoots: () => api.tryGetScanRoots(),
    triggerScan: (root) => api.tryScanFolder(root),
    readIndexStatus: () => api.tryGetIndexStatus(),
    events: sseEvents(stream),
  };
}
