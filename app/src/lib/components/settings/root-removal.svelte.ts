// Removing a watch root, or one repository under it (#247).
//
// ## The decision is the user's, and it is asked about BY NAME
//
// Removing a root used to delete everything indexed under it, whatever the
// user meant, and leave the repository rows behind. The daemon now refuses a
// removal without a decision:
//
// - keep: stop syncing, leave the data readable (the root is paused);
// - remove: prune every folder, repository and emptied project under it.
//
// So the controller lists what will stop syncing FIRST, and only then offers
// the two choices. A list that failed to load offers no decision: answering for
// data you cannot see is not consent.
//
// ## Removing one repository keeps it out
//
// The prune adds the repository to its root's exclusions, so the next scan does
// not quietly bring back what was just removed. The message says so, because
// the old "exclude" removed a repository for exactly one scan.

import type { ApiResult } from '$lib/api.js';
import type {
  PruneReport,
  PruneResult,
  RootRemovalDecision,
  RootRemovalResult,
  RootRepository,
} from '$lib/types.js';

export interface RootRemovalApi {
  tryGetRootRepositories: (id: string) => Promise<ApiResult<RootRepository[]>>;
  tryRemoveWatchRoot: (
    id: string,
    decision: RootRemovalDecision,
  ) => Promise<ApiResult<RootRemovalResult>>;
  tryPruneRepository: (path: string) => Promise<ApiResult<PruneResult>>;
}

function count(n: number, one: string, many: string): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}

/** What a prune removed, naming only the parts that happened. */
export function describePrune(r: PruneReport): string {
  const parts = [
    r.folders > 0 ? count(r.folders, 'folder', 'folders') : null,
    r.repositories > 0 ? count(r.repositories, 'repository', 'repositories') : null,
    r.projects > 0 ? count(r.projects, 'empty project', 'empty projects') : null,
  ].filter((p): p is string => p !== null);
  if (parts.length === 0) return 'Nothing was indexed there.';
  const list = parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}`;
  return `Removed ${list}.`;
}

function failureText(error: { status: number; message: string }): string {
  return error.status === 0
    ? `Could not reach the daemon — ${error.message}`
    : `The daemon returned ${error.status} ${error.message}`;
}

export class RootRemoval {
  #id: string;
  #api: RootRemovalApi;

  repositories = $state<RootRepository[]>([]);
  loaded = $state(false);
  busy = $state(false);
  failure = $state<string | null>(null);
  message = $state<string | null>(null);
  /** What the root removal did, once it has. `null` until then — and after a
   *  failure, so the screen keeps the root. */
  outcome = $state<'kept' | 'removed' | null>(null);

  constructor(rootId: string, api: RootRemovalApi) {
    this.#id = rootId;
    this.#api = api;
  }

  /** A decision needs the list it is about. */
  get canDecide(): boolean {
    return this.loaded && this.failure === null && !this.busy;
  }

  get question(): string {
    if (this.repositories.length === 0) return 'Nothing has been found under this root yet.';
    return `${count(this.repositories.length, 'repository', 'repositories')} under this root will stop syncing.`;
  }

  async open() {
    this.failure = null;
    this.message = null;
    const res = await this.#api.tryGetRootRepositories(this.#id);
    if (res.ok) {
      this.repositories = res.data;
      this.loaded = true;
    } else {
      this.failure = failureText(res.error);
    }
  }

  async decide(decision: RootRemovalDecision) {
    this.busy = true;
    this.failure = null;
    const res = await this.#api.tryRemoveWatchRoot(this.#id, decision);
    this.busy = false;
    if (!res.ok) {
      this.failure = failureText(res.error);
      return;
    }
    if (res.data.kept) {
      this.outcome = 'kept';
      this.message = 'Stopped syncing. Everything already indexed stays readable.';
    } else {
      this.outcome = 'removed';
      this.message = describePrune(res.data.pruned);
    }
  }

  async pruneRepository(path: string) {
    this.busy = true;
    this.failure = null;
    const res = await this.#api.tryPruneRepository(path);
    this.busy = false;
    if (!res.ok) {
      this.failure = failureText(res.error);
      return;
    }
    const name = this.repositories.find((r) => r.path === path)?.name ?? res.data.excluded;
    this.repositories = this.repositories.filter((r) => r.path !== path);
    this.message = `${describePrune(res.data.pruned)} ${name} is now skipped, so a rescan will not bring it back.`;
  }
}
