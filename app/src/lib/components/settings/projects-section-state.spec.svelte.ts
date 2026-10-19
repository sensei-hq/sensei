// Settings · Projects — what the screen shows and how a role is saved (#271).
//
// The defects this pins: every FOLDER of a project was listed as a repository
// (a monorepo's 18 workspace modules read as "19 repos"); a role set here only
// changed local state, so it was gone on reload; a role the daemon stored but
// the picker did not list (tool, website, …) showed as blank; and a failed load
// read as "no projects".
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { ROLES } from '$lib/setup/types.js';
import type { DaemonProject, DaemonProjectFolder } from '$lib/setup/contracts.js';
import {
  matchesSearch,
  partitionProjects,
  ProjectsSectionState,
  repositoriesOf,
} from './projects-section-state.svelte.js';

const folder = (over: Partial<DaemonProjectFolder>): DaemonProjectFolder => ({
  id: 'f',
  name: 'f',
  path: '/w/f',
  kind: 'folder',
  role: null,
  ...over,
});

const project = (over: Partial<DaemonProject>): DaemonProject => ({
  id: 'p',
  name: 'p',
  description: null,
  client: null,
  goal: null,
  stack: { languages: [], frameworks: [], runtimes: [], services: [] },
  icon: { kind: 'kanji', value: '型' },
  folders: [],
  ...over,
});

const sensei = project({
  id: 'sensei',
  name: 'sensei',
  folders: [
    folder({ id: 'root', name: 'sensei', path: '/w/sensei', kind: 'git' }),
    folder({ id: 'mod', name: 'senseid', path: '/w/sensei/crates/senseid', kind: 'module' }),
    folder({ id: 'dir', name: 'src', path: '/w/sensei/crates/senseid/src', kind: 'folder' }),
    folder({ id: 'sub', name: 'vendored', path: '/w/sensei/vendored', kind: 'subtree' }),
  ],
});

describe('repositoriesOf', () => {
  it('lists only git checkouts and subtrees, never modules or plain folders', () => {
    expect(repositoriesOf(sensei).map((f) => f.id)).toEqual(['root', 'sub']);
  });
});

describe('partitionProjects', () => {
  it('separates projects with a repository on this machine from those without', () => {
    const stale = project({ id: 'stale', folders: [folder({ kind: 'module' })] });
    const { placed, unplaced } = partitionProjects([sensei, stale]);
    expect(placed.map((p) => p.id)).toEqual(['sensei']);
    expect(unplaced.map((p) => p.id)).toEqual(['stale']);
  });
});

describe('matchesSearch', () => {
  it('matches the project name, a repository name or a repository path, any case', () => {
    expect(matchesSearch(sensei, 'SENS')).toBe(true);
    expect(matchesSearch(sensei, 'vendored')).toBe(true);
    expect(matchesSearch(sensei, '/w/sensei/vend')).toBe(true);
  });

  it('does not match on a folder that is not a repository', () => {
    expect(matchesSearch(sensei, 'crates/senseid/src')).toBe(false);
  });

  it('matches everything when the query is blank', () => {
    expect(matchesSearch(sensei, '   ')).toBe(true);
  });
});

describe('ROLES', () => {
  it('offers exactly the roles the daemon can store (sensei.folder_role)', () => {
    // The picker listed 5 of the enum's values, so a stored `tool` or `website`
    // rendered as blank. Reading the DDL keeps the two from drifting again.
    const ddl = readFileSync(
      resolve(__dirname, '../../../../../database/ddl/enum/sensei/folder_role.ddl'),
      'utf8',
    );
    const stored = [...ddl.matchAll(/'([a-z]+)'/g)].map((m) => m[1]).sort();
    expect(ROLES.map((r) => r.id).sort()).toEqual(stored);
  });
});

describe('ProjectsSectionState', () => {
  const okApi = (projects: DaemonProject[]) => ({
    tryListProjects: async () => ({ ok: true as const, data: projects }),
    tryUpdateFolder: async () => ({ ok: true as const, data: undefined }),
    tryUpdateProject: async () => ({ ok: true as const, data: undefined }),
  });

  it('reports a failed load as an error, not as no projects', async () => {
    const s = new ProjectsSectionState();
    await s.load({
      tryListProjects: async () => ({ ok: false as const, error: { status: 500, message: 'boom' } }),
      tryUpdateFolder: async () => ({ ok: true as const, data: undefined }),
      tryUpdateProject: async () => ({ ok: true as const, data: undefined }),
    });
    expect(s.error).toContain('boom');
    expect(s.loading).toBe(false);
  });

  it('saves a role to the daemon when it changes', async () => {
    const saved: Array<[string, string | null]> = [];
    const api = {
      ...okApi([sensei]),
      tryUpdateFolder: async (id: string, patch: { role?: string | null }) => {
        saved.push([id, patch.role ?? null]);
        return { ok: true as const, data: undefined };
      },
    };
    const s = new ProjectsSectionState();
    await s.load(api);
    await s.setRole(api, 'root', 'backend');
    expect(saved).toEqual([['root', 'backend']]);
    expect(repositoriesOf(s.projects[0])[0].role).toBe('backend');
    expect(s.roleError).toBeNull();
  });

  it('rolls a role back and says so when the save fails', async () => {
    const api = {
      ...okApi([project({ ...sensei, folders: [folder({ id: 'root', kind: 'git', role: 'library' })] })]),
      tryUpdateFolder: async () => ({ ok: false as const, error: { status: 400, message: 'Bad Request' } }),
    };
    const s = new ProjectsSectionState();
    await s.load(api);
    await s.setRole(api, 'root', 'quality');
    expect(repositoriesOf(s.projects[0])[0].role).toBe('library');
    expect(s.roleError).toContain('Bad Request');
  });

  it('filters the placed and unplaced lists by the search query', async () => {
    const stale = project({ id: 'stale', name: 'old-thing', folders: [] });
    const s = new ProjectsSectionState();
    await s.load(okApi([sensei, stale]));
    s.query = 'old';
    expect(s.placed.map((p) => p.id)).toEqual([]);
    expect(s.unplaced.map((p) => p.id)).toEqual(['stale']);
  });

  it('saves a rename to the daemon, and rolls it back when the save fails', async () => {
    const saved: Array<[string, unknown]> = [];
    let fail = false;
    const api = {
      ...okApi([sensei]),
      tryUpdateProject: async (id: string, patch: object) => {
        saved.push([id, patch]);
        return fail
          ? { ok: false as const, error: { status: 500, message: 'down' } }
          : { ok: true as const, data: undefined };
      },
    };
    const s = new ProjectsSectionState();
    await s.load(api);
    await s.rename(api, 'sensei', 'sensei core');
    expect(saved).toEqual([['sensei', { name: 'sensei core' }]]);
    expect(s.projects[0].name).toBe('sensei core');

    fail = true;
    await s.rename(api, 'sensei', 'oops');
    expect(s.projects[0].name).toBe('sensei core');
    expect(s.renameError).toContain('down');
  });
});
