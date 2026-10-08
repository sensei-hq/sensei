// Removing a root, or one repository under it (#247).
//
// The daemon used to delete a root's data whatever the user wanted, and left
// its repository rows behind. The endpoint now REQUIRES a decision — keep the
// data and stop syncing, or remove everything — so this controller's job is to
// show what will stop syncing BEFORE asking, and to report what was removed
// rather than assume it.

import { describe, expect, it } from 'vitest';
import { RootRemoval, describePrune, type RootRemovalApi } from './root-removal.svelte.js';
import type { PruneReport, RootRepository } from '$lib/types.js';

const repos: RootRepository[] = [
  { name: 'a', path: '/r/a', kind: 'git' },
  { name: 'b', path: '/r/b', kind: 'standalone' },
];

const report = (over: Partial<PruneReport> = {}): PruneReport => ({
  folders: 3,
  repositories: 1,
  projects: 1,
  ...over,
});

function api(over: Partial<RootRemovalApi> = {}) {
  const calls: string[] = [];
  const a: RootRemovalApi = {
    tryGetRootRepositories: async (id) => {
      calls.push(`list ${id}`);
      return { ok: true, data: repos };
    },
    tryRemoveWatchRoot: async (id, decision) => {
      calls.push(`remove ${id} ${decision}`);
      return {
        ok: true,
        data: decision === 'keep' ? { ok: true, kept: true } : { ok: true, kept: false, pruned: report() },
      };
    },
    tryPruneRepository: async (path) => {
      calls.push(`prune ${path}`);
      return { ok: true, data: { ok: true, excluded: 'a', pruned: report() } };
    },
    ...over,
  };
  return { a, calls };
}

describe('RootRemoval', () => {
  it('asks only after showing what will stop syncing', async () => {
    const { a, calls } = api();
    const r = new RootRemoval('r1', a);
    await r.open();
    expect(calls).toEqual(['list r1']);
    expect(r.repositories).toEqual(repos);
    expect(r.question).toBe('2 repositories under this root will stop syncing.');
  });

  it('says so when nothing has been found under the root yet', async () => {
    const { a } = api({ tryGetRootRepositories: async () => ({ ok: true, data: [] }) });
    const r = new RootRemoval('r1', a);
    await r.open();
    expect(r.question).toBe('Nothing has been found under this root yet.');
  });

  // Asking keep-or-remove about a list that failed to load would be asking
  // about nothing, and the user would answer for data they cannot see.
  it('a list that failed to load is a failure, and offers no decision', async () => {
    const { a } = api({
      tryGetRootRepositories: async () => ({ ok: false, error: { status: 500, message: 'boom' } }),
    });
    const r = new RootRemoval('r1', a);
    await r.open();
    expect(r.failure).toContain('500');
    expect(r.canDecide).toBe(false);
  });

  it('keep pauses the root and says the data stays', async () => {
    const { a, calls } = api();
    const r = new RootRemoval('r1', a);
    await r.open();
    await r.decide('keep');
    expect(calls).toContain('remove r1 keep');
    expect(r.outcome).toBe('kept');
    expect(r.message).toBe('Stopped syncing. Everything already indexed stays readable.');
  });

  it('remove reports what was actually removed', async () => {
    const { a } = api();
    const r = new RootRemoval('r1', a);
    await r.open();
    await r.decide('remove');
    expect(r.outcome).toBe('removed');
    expect(r.message).toBe('Removed 3 folders, 1 repository and 1 empty project.');
  });

  // The old UI dropped the row whether or not the delete worked.
  it('a failed removal keeps the root and says why', async () => {
    const { a } = api({
      tryRemoveWatchRoot: async () => ({ ok: false, error: { status: 0, message: 'connection refused' } }),
    });
    const r = new RootRemoval('r1', a);
    await r.open();
    await r.decide('remove');
    expect(r.outcome).toBeNull();
    expect(r.failure).toMatch(/could not reach the daemon/i);
  });

  it('removing one repository drops it from the list and keeps it out', async () => {
    const { a, calls } = api();
    const r = new RootRemoval('r1', a);
    await r.open();
    await r.pruneRepository('/r/a');
    expect(calls).toContain('prune /r/a');
    expect(r.repositories.map((x) => x.path)).toEqual(['/r/b']);
    expect(r.message).toBe(
      'Removed 3 folders, 1 repository and 1 empty project. a is now skipped, so a rescan will not bring it back.',
    );
  });

  it('a failed repository removal leaves the list alone', async () => {
    const { a } = api({
      tryPruneRepository: async () => ({ ok: false, error: { status: 500, message: 'Internal Server Error' } }),
    });
    const r = new RootRemoval('r1', a);
    await r.open();
    await r.pruneRepository('/r/a');
    expect(r.repositories).toEqual(repos);
    expect(r.failure).toContain('500');
  });
});

describe('describePrune', () => {
  it('names only what was removed', () => {
    expect(describePrune(report({ folders: 1, repositories: 0, projects: 0 }))).toBe('Removed 1 folder.');
    expect(describePrune(report({ folders: 2, repositories: 2, projects: 0 }))).toBe(
      'Removed 2 folders and 2 repositories.',
    );
  });

  it('nothing removed is said plainly, not as "removed 0"', () => {
    expect(describePrune(report({ folders: 0, repositories: 0, projects: 0 }))).toBe(
      'Nothing was indexed there.',
    );
  });
});
