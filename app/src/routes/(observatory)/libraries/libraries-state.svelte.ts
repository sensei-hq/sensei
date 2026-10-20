// Observatory · Libraries — the screen's state: every detected library, and the
// selected one's usage. Each read is the daemon's answer or why it failed
// (#288): a list that could not be read is not "No libraries indexed.", and a
// usage read that failed is not a library that looks unused.
import { failureReason, type ApiResult } from '$lib/api.js';
import type { LibEntry } from '$lib/types.js';

export type UsageEntry = { library_name: string; folder: string; version_used: string | null; import_count: number };

/** The reads the Libraries screen makes — injected so a spec can fail them. */
export interface LibrariesApi {
  tryGetLibs: (params?: { repoId?: string; projectId?: string; shared?: boolean }) => Promise<ApiResult<{ total: number; libs: LibEntry[] }>>;
  tryGetLibraryUsage: (id: string) => Promise<ApiResult<{ usage: UsageEntry[] }>>;
}

export class LibrariesState {
  libs = $state<LibEntry[]>([]);
  loading = $state(true);
  error = $state<string | null>(null);
  selected = $state<LibEntry | null>(null);
  usage = $state<UsageEntry[]>([]);
  usageLoading = $state(false);
  usageError = $state<string | null>(null);

  /** Every detected library with its usage count (repoCount) — not just those
   *  shared across ≥2 repos: `shared: true` hid single-repo libs entirely, which
   *  is why most @rokkit/* packages never appeared (#63). */
  async load(api: LibrariesApi): Promise<void> {
    this.loading = true;
    const read = await api.tryGetLibs({});
    this.libs = read.ok ? read.data.libs : [];
    this.error = read.ok ? null : failureReason(read.error);
    this.loading = false;
  }

  /** Select a library and read its usage. An answer that arrives after the
   *  selection moved on is dropped. */
  async select(api: LibrariesApi, lib: LibEntry): Promise<void> {
    this.selected = lib;
    this.usage = [];
    this.usageError = null;
    this.usageLoading = true;
    const read = await api.tryGetLibraryUsage(lib.id);
    if (this.selected?.id !== lib.id) return;
    this.usage = read.ok ? read.data.usage : [];
    this.usageError = read.ok ? null : failureReason(read.error);
    this.usageLoading = false;
  }
}
