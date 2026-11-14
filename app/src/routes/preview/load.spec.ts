import { describe, it, expect } from 'vitest';
import { load as indexLoad } from './+page.js';
import { load as componentLoad } from './[component]/+page.js';

// The preview is a development page (DEV is true under vitest): the index lists
// the catalogue's components; a component's page gets its fixtures and the view
// the query asks for; an unknown component is a 404, never an empty page.
const call = <T>(fn: (e: never) => T, path: string, params: Record<string, string> = {}) =>
  fn({ url: new URL(`http://localhost${path}`), params } as never);

describe('the preview routes', () => {
  it('the index lists every component with its fixture count', async () => {
    const data = (await call(indexLoad, '/preview')) as { components: { name: string; count: number }[] };
    const names = data.components.map((c) => c.name);
    expect(names).toContain('WaitList');
    expect(names).toContain('RelayShell');
    expect(new Set(names).size).toBe(names.length);
    expect(data.components.find((c) => c.name === 'WaitList')!.count).toBeGreaterThanOrEqual(2);
  });

  it('a component’s page carries its fixtures and the view from the query', async () => {
    const data = (await call(componentLoad, '/preview/WaitList?theme=dark&fixture=nudged-by-you', { component: 'WaitList' })) as {
      component: string;
      entries: { id: string }[];
      view: { theme: string; fixture: string | null };
    };
    expect(data.component).toBe('WaitList');
    expect(data.entries.every((x) => x.id.startsWith('WaitList/'))).toBe(true);
    expect(data.view).toMatchObject({ theme: 'dark', fixture: 'nudged-by-you' });
  });

  it('all is every fixture, for the capture matrix', async () => {
    const all = (await call(componentLoad, '/preview/all', { component: 'all' })) as { entries: unknown[] };
    const index = (await call(indexLoad, '/preview')) as { components: { count: number }[] };
    expect(all.entries).toHaveLength(index.components.reduce((n, c) => n + c.count, 0));
  });

  it('an unknown component is a 404', async () => {
    await expect(call(componentLoad, '/preview/Nope', { component: 'Nope' })).rejects.toMatchObject({ status: 404 });
  });
});
