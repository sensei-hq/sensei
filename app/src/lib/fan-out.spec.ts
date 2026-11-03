import { describe, it, expect } from 'vitest';
import { fanOutWorkspaces, unreadNotice } from './fan-out.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = (message: string) => ({ ok: false as const, error: { status: 502, message } });

// #288: a cross-workspace screen keeps what it read and names what it could not —
// a workspace whose read failed is not silently a clean workspace.
describe('fanOutWorkspaces', () => {
  it('keeps each read with its workspace, and names each workspace it could not read', async () => {
    const workspaces = [{ id: 'a', name: 'alpha' }, { id: 'b', name: 'beta' }, { id: 'c', name: 'gamma' }];
    const { read, unread } = await fanOutWorkspaces(workspaces, async (p) =>
      p.id === 'b' ? down('Bad Gateway') : ok([p.id.toUpperCase()]),
    );
    expect(read.map((r) => [r.workspace.name, r.data])).toEqual([['alpha', ['A']], ['gamma', ['C']]]);
    expect(unread).toEqual([{ name: 'beta', reason: 'Bad Gateway' }]);
  });

  it('reads nothing for no workspaces', async () => {
    expect(await fanOutWorkspaces([], async () => ok(1))).toEqual({ read: [], unread: [] });
  });
});

describe('unreadNotice — a partial read, through the shared Notice', () => {
  it('says how many could not be read, each with why, and that the screen is partial', () => {
    expect(unreadNotice([{ name: 'alpha', reason: 'Bad Gateway' }, { name: 'beta', reason: 'HTTP 500' }])).toEqual({
      tone: 'fault',
      title: "Couldn't read 2 workspaces",
      detail: 'alpha (Bad Gateway), beta (HTTP 500)',
      remedy: 'What this screen shows leaves them out.',
    });
  });

  it('one in the singular; none, no notice', () => {
    expect(unreadNotice([{ name: 'a', reason: 'x' }])!.title).toBe("Couldn't read 1 workspace");
    expect(unreadNotice([])).toBeNull();
  });
});
