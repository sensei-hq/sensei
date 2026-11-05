import { describe, it, expect, vi } from 'vitest';
import { RepoElections, type ElectionsApi } from './repo-elections-state.svelte.js';
import type { DojoRepositoryRow } from '$lib/types.js';

// WP8 (L0 ruling: the app toggle is L2's): the switch reads the dōjō's ruling and
// asks it to record a change; a refused change is a refusal Notice with the
// dōjō's reason, and a failed read is an error — never "nothing to change".

const ok = <T>(data: T) => Promise.resolve({ ok: true as const, data });
const fail = (status: number, message: string, body?: Record<string, unknown>) =>
  Promise.resolve({ ok: false as const, error: { status, message, body } });

const NOTES: DojoRepositoryRow = { repo_key: 'github.com/me/notes', elected: true, sync_enabled: true, configurable_by_me: true };
const WEB: DojoRepositoryRow = { repo_key: 'github.com/acme/web', elected: true, sync_enabled: true, configurable_by_me: false };

function setup(over: Partial<Record<keyof ElectionsApi, unknown>> = {}) {
  const api = {
    tryGetDojoRepositories: vi.fn(() => ok({ repositories: [NOTES, WEB] })),
    tryPatchRepositoryElection: vi.fn(() => ok({ repo_key: NOTES.repo_key, elected: false, sync_enabled: false })),
    ...over,
  } as unknown as ElectionsApi & Record<string, ReturnType<typeof vi.fn>>;
  return { api, s: new RepoElections(api, () => 'slot-1') };
}

describe('RepoElections', () => {
  it.skip('pending: wp8-daemon — reads the identity’s rulings by repo key', async () => {
    const { api, s } = setup();
    await s.load();
    expect(api.tryGetDojoRepositories).toHaveBeenCalledWith('slot-1');
    expect(s.rows?.get(NOTES.repo_key)).toEqual(NOTES);
    expect(s.rows?.get(WEB.repo_key)?.configurable_by_me).toBe(false);
    expect(s.error).toBeNull();
  });

  it.skip('pending: wp8-daemon — a failed read is an error, never an empty ruling', async () => {
    const { s } = setup({ tryGetDojoRepositories: vi.fn(() => fail(502, 'dōjō returned 500')) });
    await s.load();
    expect(s.rows).toBeNull();
    expect(s.error).toBe('dōjō returned 500');
  });

  it.skip('pending: wp8-daemon — a change asks the dōjō, then shows what it recorded', async () => {
    const { api, s } = setup();
    await s.load();
    await s.toggle(NOTES.repo_key, false);
    expect(api.tryPatchRepositoryElection).toHaveBeenCalledWith(NOTES.repo_key, 'slot-1', false);
    expect(s.rows?.get(NOTES.repo_key)).toEqual({ ...NOTES, elected: false, sync_enabled: false });
    expect(s.notice).toBeNull();
  });

  it.skip('pending: wp8-daemon — a refused change (409) is a refusal Notice with the dōjō’s reason; nothing flips', async () => {
    const { s } = setup({
      tryPatchRepositoryElection: vi.fn(() =>
        fail(409, 'not_yours_to_change', { error: 'not_yours_to_change', repo_key: WEB.repo_key, reason: 'only an admin may change this' })),
    });
    await s.load();
    await s.toggle(WEB.repo_key, false);
    expect(s.notice).toEqual({
      tone: 'refusal',
      title: 'Only this organisation’s admin can change this repository’s sync',
      detail: 'only an admin may change this',
      code: 'not_yours_to_change',
    });
    expect(s.rows?.get(WEB.repo_key)).toEqual(WEB);
  });

  it.skip('pending: wp8-daemon — a change that failed (502) is a fault Notice; nothing flips', async () => {
    const { s } = setup({ tryPatchRepositoryElection: vi.fn(() => fail(502, 'dōjō returned 500')) });
    await s.load();
    await s.toggle(NOTES.repo_key, false);
    expect(s.notice).toEqual({
      tone: 'fault',
      title: 'Couldn’t change this repository’s sync',
      detail: 'dōjō returned 500',
      code: 'election_failed',
    });
    expect(s.rows?.get(NOTES.repo_key)).toEqual(NOTES);
  });
});
