import { describe, it, expect, vi, afterEach } from 'vitest';
import { load } from './+page';

afterEach(() => vi.unstubAllGlobals());

const reply = (body: unknown, status = 200) =>
  ({ ok: status < 400, status, statusText: status < 400 ? 'OK' : 'Internal Server Error', json: async () => body }) as Response;

/** Every read answers, except URLs containing `failing`. */
function daemon(failing: string) {
  vi.stubGlobal('fetch', vi.fn(async (url: string) => {
    if (String(url).includes(failing)) return reply({}, 500);
    if (String(url).includes('/metrics/registry')) return reply({ metrics: [{ key: 'ftr', formula: 'f' }], count: 1 });
    if (String(url).includes('/recommendations')) return reply([]);
    if (String(url).includes('/metrics/ftr')) return reply({ metric: 'ftr', grain: 'daily', formula: 'f', series: [], count: 0 });
    return reply({ metrics: [], count: 0 });
  }));
}

const run = () =>
  (load as unknown as (e: unknown) => Promise<Record<string, unknown>>)({
    params: { id: 'p1', key: 'ftr' },
    url: new URL('http://x/workspace/p1/metrics/ftr'),
  });

describe('metric detail load', () => {
  it('reads the registry’s formula', async () => {
    daemon('nothing-fails');
    const d = await run();
    expect(d.error).toBeNull();
    expect(d.formula).toBe('f');
  });

  it('a registry that could not be read is the page’s error, not an empty catalogue (#288)', async () => {
    daemon('/metrics/registry');
    const d = await run();
    expect(d.error).toContain('Internal Server Error');
  });
});
