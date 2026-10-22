import { describe, it, expect } from 'vitest';
import { fanOutWorkspaces } from './fan-out.js';

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
