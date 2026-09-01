// Per-root scan exclusions — the translation from what someone types to what the
// watcher matches.
//
// ## Why this exists
//
// The mechanism was already complete and unreachable: `folders_to_watch.excluded`
// stores the list, `PUT /api/scan/roots/{id}` replaces it (pruning newly-excluded
// subtrees and re-scanning removed ones), and `RootWatcher::should_watch_path`
// enforces it. The Roots screen created every root with `excluded: []` and offered
// no editor, so the only way to bound a sweep was curl.
//
// MEASURED 2026-08-31: `find-me-board` carries 1,230 folders, and 1,211 of them
// sit under `docs/proposal/deck-node` — an unpacked Node source tarball, all
// openssl arch directories. None of it is `node_modules`, so the static
// EXCLUDE_DIRS list correctly does not touch it, and a non-git folder has no
// `.gitignore` to bound it either.
//
// ## The two forms are not alike
//
// `should_watch_path` accepts an ABSOLUTE path (a subtree prefix) or a BARE NAME
// (any path segment, anywhere under the root). They look almost identical when
// typed and behave very differently — excluding `docs` to stop one vendored tree
// would silently drop every `docs` folder under the root. So the UI states which
// one it is before it is saved, rather than after someone notices what went.

/** The result of validating what a user typed. */
export type Normalised =
  | { ok: true; value: string; error?: undefined }
  | { ok: false; value?: undefined; error: string };

/**
 * Validate and canonicalise one exclusion entry against its root.
 *
 * An absolute path OUTSIDE the root is refused rather than stored. Exclusions are
 * compared as a prefix of paths under that root, so a path elsewhere can never
 * match: storing it would look like it worked and change nothing, which is the
 * worst outcome for a control whose whole job is to stop a sweep.
 *
 * A trailing slash is trimmed because the watcher trims it too — keeping it would
 * make the stored value differ from the compared one, and only the stored value
 * is what the user sees.
 */
export function normaliseExclusion(input: string, rootPath: string): Normalised {
  const trimmed = input.trim();
  if (!trimmed) return { ok: false, error: 'Enter a folder name or a path to exclude.' };

  const root = rootPath.replace(/\/+$/, '');
  // `~` is expanded against the ROOT rather than the process home: this runs in a
  // webview with no home of its own, and a `~` a user types here means "inside
  // the place I am configuring".
  const expanded = trimmed.startsWith('~/')
    ? `${root.slice(0, root.lastIndexOf('/'))}/${trimmed.slice(2)}`
    : trimmed;

  if (!expanded.startsWith('/')) return { ok: true, value: expanded };

  const value = expanded.replace(/\/+$/, '');
  if (value !== root && !value.startsWith(`${root}/`)) {
    return {
      ok: false,
      error: `An absolute exclusion must be inside ${root} — a path elsewhere would never match.`,
    };
  }
  return { ok: true, value };
}

/** Plain-language statement of what one entry will exclude. Shown BEFORE saving,
 *  because the bare-name form is far broader than it looks. */
export function describeExclusion(value: string, rootPath: string): string {
  if (!value.startsWith('/')) {
    return `Skips every folder named "${value}" anywhere under this root.`;
  }
  const root = rootPath.replace(/\/+$/, '');
  const relative = value.startsWith(`${root}/`) ? value.slice(root.length + 1) : value;
  return `Skips ${relative} and everything under it.`;
}

/** Pure: the list with `value` present exactly once. */
export function withExclusion(current: readonly string[], value: string): string[] {
  return current.includes(value) ? [...current] : [...current, value];
}

/** Pure: the list without `value`. */
export function withoutExclusion(current: readonly string[], value: string): string[] {
  return current.filter((e) => e !== value);
}

/** The API surface this controller needs — injected, so it tests without a daemon. */
export interface RootExclusionsApi {
  updateWatchRoot: (
    id: string,
    excluded: string[],
  ) => Promise<
    { ok: true; data: { excluded: string[] } } | { ok: false; error: { message: string } }
  >;
}

/**
 * One root's exclusion list.
 *
 * Every change is a FULL REPLACE, because that is what the endpoint is: it diffs
 * the new list against the old to decide what to prune and what to re-scan. Sending
 * a delta would leave it unable to tell a removal from an omission.
 *
 * The daemon's echoed list is adopted rather than the one sent, and a failed write
 * leaves the list untouched — an exclusion that appears set but is not means the
 * sweep continues while the screen says it stopped.
 */
export class RootExclusions {
  excluded = $state<string[]>([]);
  saving = $state(false);
  error = $state<string | null>(null);

  #id: string;
  #rootPath: string;
  #api: RootExclusionsApi;

  constructor(id: string, rootPath: string, excluded: string[], api: RootExclusionsApi) {
    this.#id = id;
    this.#rootPath = rootPath;
    this.excluded = [...excluded];
    this.#api = api;
  }

  /** What `value` will exclude, in plain language. */
  describe(value: string): string {
    return describeExclusion(value, this.#rootPath);
  }

  /** Validate then add. Returns false — without contacting the daemon — when the
   *  entry could never match. */
  async add(input: string): Promise<boolean> {
    const parsed = normaliseExclusion(input, this.#rootPath);
    if (!parsed.ok) {
      this.error = parsed.error;
      return false;
    }
    return this.#replace(withExclusion(this.excluded, parsed.value));
  }

  async remove(value: string): Promise<boolean> {
    return this.#replace(withoutExclusion(this.excluded, value));
  }

  async #replace(next: string[]): Promise<boolean> {
    if (this.saving) return false; // one full-replace at a time
    this.saving = true;
    this.error = null;
    const res = await this.#api.updateWatchRoot(this.#id, next);
    this.saving = false;
    if (!res.ok) {
      this.error = res.error.message; // list untouched → the control reverts
      return false;
    }
    this.excluded = res.data.excluded; // adopt what the daemon stored
    return true;
  }
}
