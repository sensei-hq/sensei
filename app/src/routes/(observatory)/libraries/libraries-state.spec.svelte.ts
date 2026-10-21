import { describe, it, expect } from 'vitest';
import { LibrariesState, type LibrariesApi } from './libraries-state.svelte.js';
import type { LibEntry } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const lib = (id: string, name = id) => ({ id, name, repoCount: 1, repos: [] }) as unknown as LibEntry;
const usage = [{ library_name: 'rokkit', folder: 'app', version_used: '1.0', import_count: 4 }];

const api = (over: Partial<LibrariesApi> = {}): LibrariesApi => ({
  tryGetLibs: async () => ok({ total: 2, libs: [lib('a', 'rokkit'), lib('b', 'zod')] }),
  tryGetLibraryUsage: async () => ok({ usage }),
  ...over,
});

// #288: the libraries and a library's usage, or why they could not be read —
// not "No libraries indexed." and not a library that looks unused.
describe('LibrariesState', () => {
  it('loads every detected library', async () => {
    const s = new LibrariesState();
    await s.load(api());
    expect([s.loading, s.error, s.libs.map((l) => l.name)]).toEqual([false, null, ['rokkit', 'zod']]);
  });

  it('a failed list read is an error, not an empty index', async () => {
    const s = new LibrariesState();
    await s.load(api({ tryGetLibs: async () => down }));
    expect([s.loading, s.error, s.libs]).toEqual([false, 'Service Unavailable', []]);
  });

  it('selecting a library reads its usage', async () => {
    const s = new LibrariesState();
    await s.select(api(), lib('a', 'rokkit'));
    expect([s.selected?.id, s.usage, s.usageError, s.usageLoading]).toEqual(['a', usage, null, false]);
  });

  it('a failed usage read is said so — not a library that looks unused', async () => {
    const s = new LibrariesState();
    await s.select(api({ tryGetLibraryUsage: async () => down }), lib('a'));
    expect([s.usage, s.usageError]).toEqual([[], 'Service Unavailable']);
  });

  it('a stale usage answer for a previous selection is dropped', async () => {
    const s = new LibrariesState();
    let release!: () => void;
    const slow = api({ tryGetLibraryUsage: (id) => (id === 'a' ? new Promise((r) => (release = () => r(ok({ usage })))) : Promise.resolve(ok({ usage: [] }))) });
    const first = s.select(slow, lib('a'));
    await s.select(slow, lib('b'));
    release();
    await first;
    expect([s.selected?.id, s.usage]).toEqual(['b', []]);
  });
});
