import { describe, it, expect, vi, afterEach } from 'vitest';
import { load } from './+page';

afterEach(() => vi.unstubAllGlobals());

const reply = (body: unknown, status = 200) =>
  ({ ok: status < 400, status, statusText: status < 400 ? 'OK' : 'Internal Server Error', json: async () => body }) as Response;

/** Every read answers, except URLs containing `failing`. */
function daemon(failing: string) {
  vi.stubGlobal('fetch', vi.fn(async (url: string) => {
    if (String(url).includes(failing)) return reply({}, 500);
    if (String(url).includes('/metrics/registry')) return reply({ metrics: [{ key: 'ftr', family: 'flow' }], count: 1 });
    if (String(url).includes('consent')) return reply([]);
    if (String(url).includes('correlations')) return reply({ cells: 0, min_pairs: 20, correlations: [] });
    return reply({ metrics: [], count: 0 });
  }));
}

const run = () =>
  (load as unknown as (e: unknown) => Promise<Record<string, unknown>>)({ params: { id: 'p1' } });

describe('metrics landing load', () => {
  it('reads the registry’s families', async () => {
    daemon('nothing-fails');
    const d = await run();
    expect(d.error).toBeNull();
    expect(d.registry).toEqual([{ key: 'ftr', family: 'flow' }]);
  });

  it('a registry that could not be read is the page’s error, not an ungrouped grid (#288)', async () => {
    daemon('/metrics/registry');
    const d = await run();
    expect(d.error).toContain('Internal Server Error');
  });
});
