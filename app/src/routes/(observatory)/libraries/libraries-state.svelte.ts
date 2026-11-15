// Observatory · Libraries — the screen's state: every detected library, the
// search, the selected library and its usage. Each read is the daemon's answer
// or why it failed (#288): a list that could not be read is not "No libraries
// indexed.", and a usage read that failed is not a library that looks unused.
//
// The list itself is the loader's read (+page.ts → readLibraries); this class
// owns what follows it, including Retry, which re-reads through that same read
// rather than invalidating every load on the route.
import { failureReason, type ApiResult } from '$lib/api.js';
import type { LibCapability, LibEntry, LibPage } from '$lib/types.js';
import { readLibraries, type LibrariesRead } from './libraries-load.js';

export type UsageEntry = { library_name: string; folder: string; version_used: string | null; import_count: number };

/** The reads the Libraries screen makes — injected so a spec can fail them. */
export interface LibrariesApi {
  tryGetLibs: (params?: { repoId?: string; workspaceId?: string; shared?: boolean }) => Promise<ApiResult<{ total: number; libs: LibEntry[] }>>;
  tryGetLibraryUsage: (id: string) => Promise<ApiResult<{ usage: UsageEntry[] }>>;
  tryGetLibrarySkills: (ref: string) => Promise<ApiResult<LibCapability[]>>;
  tryGetLibraryAgents: (ref: string) => Promise<ApiResult<LibCapability[]>>;
  tryGetLibraryDocs: (ref: string) => Promise<ApiResult<LibPage[]>>;
}

/** What a library provides from its own `sensei.library.json` (#341): its docs,
 *  skills and agents — or why they could not be read. */
export interface Provided {
  docs: LibPage[];
  skills: LibCapability[];
  agents: LibCapability[];
  loading: boolean;
  error: string | null;
}

const NONE: Provided = { docs: [], skills: [], agents: [], loading: false, error: null };

export class LibrariesState {
  libs = $state<LibEntry[]>([]);
  /** True only while a Retry re-reads; the loader's read has already landed. */
  loading = $state(false);
  error = $state<string | null>(null);
  /** The search box's value — a write, so it lives here, not in the template. */
  search = $state('');
  selected = $state<LibEntry | null>(null);
  usage = $state<UsageEntry[]>([]);
  usageLoading = $state(false);
  usageError = $state<string | null>(null);
  provided = $state<Provided>({ ...NONE });

  /** Seeded from the loader's read, so the page draws the list on first paint. */
  constructor(read: LibrariesRead = { libs: [], error: null }) {
    this.setRead(read);
  }

  setRead(read: LibrariesRead): void {
    this.libs = read.libs;
    this.error = read.error;
  }

  /** The rows the search leaves. Every library when it is empty (#63: not only
   *  those shared across ≥2 repos). Same rule as before the move: a
   *  case-insensitive substring of the name, whitespace included. */
  get visible(): LibEntry[] {
    const q = this.search.toLowerCase();
    return q ? this.libs.filter((l) => l.name.toLowerCase().includes(q)) : this.libs;
  }

  /** Re-read every detected library, for Retry — not a full route invalidate. */
  async load(api: LibrariesApi): Promise<void> {
    this.loading = true;
    const read = await readLibraries(api);
    this.setRead(read);
    this.loading = false;
  }

  /** Select a library and read its usage. An answer that arrives after the
   *  selection moved on is dropped. */
  async select(api: LibrariesApi, lib: LibEntry): Promise<void> {
    this.selected = lib;
    this.usage = [];
    this.usageError = null;
    this.usageLoading = true;
    this.provided = { ...NONE, loading: true };
    await Promise.all([this.readUsage(api, lib), this.readProvided(api, lib)]);
  }

  private async readUsage(api: LibrariesApi, lib: LibEntry): Promise<void> {
    const read = await api.tryGetLibraryUsage(lib.id);
    if (this.selected?.id !== lib.id) return;
    this.usage = read.ok ? read.data.usage : [];
    this.usageError = read.ok ? null : failureReason(read.error);
    this.usageLoading = false;
  }

  /** The three reads together: one that failed makes the whole "provides"
   *  section an error, never a library that looks as if it provides nothing. */
  private async readProvided(api: LibrariesApi, lib: LibEntry): Promise<void> {
    const [docs, skills, agents] = await Promise.all([
      // By id: a name can be two libraries (`uuid` is a crate and an npm package).
      api.tryGetLibraryDocs(lib.id),
      api.tryGetLibrarySkills(lib.id),
      api.tryGetLibraryAgents(lib.id),
    ]);
    if (this.selected?.id !== lib.id) return;
    const failed = [docs, skills, agents].find((r) => !r.ok);
    this.provided =
      failed && !failed.ok
        ? { ...NONE, error: failureReason(failed.error) }
        : {
            docs: docs.ok ? docs.data : [],
            skills: skills.ok ? skills.data : [],
            agents: agents.ok ? agents.data : [],
            loading: false,
            error: null,
          };
  }
}
