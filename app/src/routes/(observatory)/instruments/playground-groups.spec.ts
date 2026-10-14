import { describe, it, expect } from 'vitest';
import { buildPlaygroundGroups, serversByFamily, inferKind } from './playground-groups.js';

const server = (id: string, acp_family: string, mcp_key: string, enabled = true) =>
  ({ id, acp_family, mcp_key, enabled });

describe('serversByFamily', () => {
  it('groups by assistant in a fixed order, servers by key within each', () => {
    const groups = serversByFamily([
      server('3', 'zed', 'b'), server('1', 'claude', 'z'), server('2', 'claude', 'a'), server('4', 'cursor', 'c'),
    ]);
    expect(groups.map((g) => g.family)).toEqual(['claude', 'cursor', 'zed']);
    expect(groups[0].servers.map((s) => s.mcp_key)).toEqual(['a', 'z']);
    expect(groups[0].label).toBe('Claude');
  });

  it('keeps a family it has no label for, after the known ones — never drops a server', () => {
    const groups = serversByFamily([server('1', 'windsurf', 'x'), server('2', 'claude', 'y')]);
    expect(groups.map((g) => g.family)).toEqual(['claude', 'windsurf']);
    expect(groups[1].label).toBe('windsurf');
    expect(groups.flatMap((g) => g.servers)).toHaveLength(2);
  });

  it('is empty for no servers', () => {
    expect(serversByFamily([])).toEqual([]);
  });
});

describe('buildPlaygroundGroups', () => {
  it('puts sensei first, then one group per discovered server with its manifest tools', () => {
    const groups = buildPlaygroundGroups(
      [{ name: 'search', kind: 'query', summary: 's' }],
      [server('srv', 'claude', 'github', false)],
      { srv: { tools: [{ name: 'list_issues', description: 'd' }, { description: 'nameless' }] } },
    );
    expect(groups.map((g) => g.id)).toEqual(['sensei', 'srv']);
    expect(groups[1]).toMatchObject({ name: 'github', installed: false, source: 'discovered', serverId: 'srv' });
    expect(groups[1].tools.map((t) => [t.name, t.kind])).toEqual([['list_issues', 'query']]);
  });
});

describe('inferKind', () => {
  it('prefers a declared kind, then reads the verb', () => {
    expect(inferKind('get_x', 'action')).toBe('action');
    expect(inferKind('delete_x', undefined)).toBe('action');
    expect(inferKind('describe_x', undefined)).toBe('query');
    expect(inferKind('frobnicate', undefined)).toBe('other');
  });
});
