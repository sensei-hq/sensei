import { describe, expect, it } from 'vitest';
import { parseServerForm } from './server-form';

describe('parseServerForm', () => {
  it('reads one argument per line and KEY=VALUE environment lines', () => {
    expect(
      parseServerForm({ name: ' github ', command: ' npx ', args: '-y\n@modelcontextprotocol/server-github\n', env: 'TOKEN=a=b\n\nDEBUG=1' }),
    ).toEqual({
      ok: true,
      server: { name: 'github', command: 'npx', args: ['-y', '@modelcontextprotocol/server-github'], env: { TOKEN: 'a=b', DEBUG: '1' } },
    });
  });

  it('refuses a missing name or command, and an environment line without =', () => {
    expect(parseServerForm({ name: '', command: 'x', args: '', env: '' })).toEqual({ ok: false, error: 'A name is required' });
    expect(parseServerForm({ name: 'x', command: ' ', args: '', env: '' })).toEqual({ ok: false, error: 'A command is required' });
    expect(parseServerForm({ name: 'x', command: 'y', args: '', env: 'NOPE' })).toEqual({
      ok: false, error: 'Environment line "NOPE" is not KEY=VALUE',
    });
  });
});
