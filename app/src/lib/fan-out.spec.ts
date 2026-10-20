import { describe, it, expect } from 'vitest';
import { fanOutProjects } from './fan-out.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = (message: string) => ({ ok: false as const, error: { status: 502, message } });

// #288: a cross-project screen keeps what it read and names what it could not —
// a project whose read failed is not silently a clean project.
describe('fanOutProjects', () => {
  it('keeps each read with its project, and names each project it could not read', async () => {
    const projects = [{ id: 'a', name: 'alpha' }, { id: 'b', name: 'beta' }, { id: 'c', name: 'gamma' }];
    const { read, unread } = await fanOutProjects(projects, async (p) =>
      p.id === 'b' ? down('Bad Gateway') : ok([p.id.toUpperCase()]),
    );
    expect(read.map((r) => [r.project.name, r.data])).toEqual([['alpha', ['A']], ['gamma', ['C']]]);
    expect(unread).toEqual([{ name: 'beta', reason: 'Bad Gateway' }]);
  });

  it('reads nothing for no projects', async () => {
    expect(await fanOutProjects([], async () => ok(1))).toEqual({ read: [], unread: [] });
  });
});
