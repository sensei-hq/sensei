import { describe, expect, it } from 'vitest';
import { load } from './+page';

const run = (lens: string) => load({ params: { id: 'w', lens } } as never);

describe('diagrams/[lens] load', () => {
  it('opens a Code map lens with its view', () => {
    expect(run('wheel')).toMatchObject({ workspaceId: 'w', lens: 'wheel', view: { label: 'Wheel' } });
  });
  it('is a 404 for a segment that is no lens — a retired page included', () => {
    for (const lens of ['layers', 'neighbourhood', 'codemap', 'nope']) {
      expect(() => run(lens), lens).toThrow(expect.objectContaining({ status: 404 }));
    }
  });
});
