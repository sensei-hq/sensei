// Settings · Dōjō · the sync switch on each repository (WP8, D46): what the
// identity's dōjō rules on its repositories (who may change each one's sync),
// and the change itself. The dōjō decides; this state only reads its ruling and
// asks it to record a change. A refused change says why — never a switch that
// silently flips back, and never a failed read shown as "nothing to change".
import type { SenseiApi } from '$lib/api.js';
import type { DojoRepositoryRow } from '$lib/types.js';
import type { ScreenNoticeProps } from './dojo-connection-state.svelte.js';

export type ElectionsApi = Pick<SenseiApi, 'tryGetDojoRepositories' | 'tryPatchRepositoryElection'>;

export class RepoElections {
  /** The dōjō's rulings by repo key; `null` until read, or when the read failed. */
  rows = $state<Map<string, DojoRepositoryRow> | null>(null);
  /** Why the rulings could not be read. */
  error = $state<string | null>(null);
  /** What the last change could not do, and why. */
  notice = $state<ScreenNoticeProps | null>(null);

  constructor(
    private readonly api: ElectionsApi,
    private readonly persona: () => string | null,
  ) {}

  /** pending: wp8-daemon */
  async load(): Promise<void> {
    throw new Error('pending: wp8-daemon');
  }

  /** pending: wp8-daemon */
  async toggle(_repoKey: string, _elected: boolean): Promise<void> {
    throw new Error('pending: wp8-daemon');
  }
}
