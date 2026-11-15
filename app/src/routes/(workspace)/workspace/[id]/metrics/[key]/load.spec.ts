import { describe, it, expect } from 'vitest';
import { load } from './+page';

// The detail's reads live in the workspace's metrics state (shared with the
// landing, kept across metrics and grains); the load only reads the URL.
const run = (search = '') =>
  (load as unknown as (e: unknown) => Record<string, unknown>)({
    params: { id: 'p1', key: 'ftr' },
    url: new URL(`http://x/workspace/p1/metrics/ftr${search}`),
  });

describe('metric detail load', () => {
  it('names the metric and the grain the URL asks for', () => {
    expect(run('?grain=weekly')).toEqual({ selectedKey: 'ftr', grain: 'weekly' });
  });

  it('a grain it does not know is daily', () => {
    expect(run('?grain=hourly')).toEqual({ selectedKey: 'ftr', grain: 'daily' });
    expect(run()).toEqual({ selectedKey: 'ftr', grain: 'daily' });
  });
});
