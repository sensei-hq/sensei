import { describe, it, expect } from 'vitest';
import { roleAssignments } from './role-meta.js';
import type { DaemonChain } from './contracts.js';

const chain = (id: string, role: DaemonChain['role']) => ({ id, role }) as DaemonChain;

describe('roleAssignments', () => {
  it('assigns each role the first chain that claims it', () => {
    expect(roleAssignments([chain('a', 'inference'), chain('b', 'inference'), chain('c', 'embedding'), chain('d', null)])).toEqual({
      inference: 'a', consolidation: null, embedding: 'c', voice: null,
    });
  });
  it('leaves every role unassigned for no chains', () => {
    expect(roleAssignments([])).toEqual({ inference: null, consolidation: null, embedding: null, voice: null });
  });
});
