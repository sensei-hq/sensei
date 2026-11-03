// L5: the Libraries read moves out of the component's onMount and into the
// route loader. The loader is a read like any other, so the #288 rule holds
// there too: a list that could not be read is the error it was, never an empty
// index that the screen would render as "No libraries indexed."
import { describe, it, expect } from 'vitest';
import type { LibEntry } from '$lib/types.js';
import type { LibrariesApi } from './libraries-state.svelte.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const lib = (id: string, name = id) => ({ id, name, repoCount: 1, repos: [] }) as unknown as LibEntry;

const api = (over: Partial<LibrariesApi> = {}): LibrariesApi => ({
  tryGetLibs: async () => ok({ total: 2, libs: [lib('a', 'rokkit'), lib('b', 'zod')] }),
  tryGetLibraryUsage: async () => ok({ usage: [] }),
  tryGetLibrarySkills: async () => ok([]),
  tryGetLibraryAgents: async () => ok([]),
  tryGetLibraryDocs: async () => ok([]),
  ...over,
});

// Phase A guard: `readLibraries` does not exist yet, so the test fails on the
// missing behaviour instead of a module-resolution error that would fail the
// whole file before any assertion ran.
async function readLibraries(api: LibrariesApi): Promise<{ libs: LibEntry[]; error: string | null }> {
  const mod = (await import('./libraries-state.svelte.js')) as unknown as {
    readLibraries?: (a: LibrariesApi) => Promise<{ libs: LibEntry[]; error: string | null }>;
  };
  if (!mod.readLibraries) throw new Error('readLibraries is not exported (the read has not moved to the loader yet)');
  return mod.readLibraries(api);
}

describe('readLibraries — what the Libraries route load returns', () => {
  it.skip('pending: L5 libraries — the read moves to the loader: returns every detected library', async () => {
    const read = await readLibraries(api());
    expect([read.error, read.libs.map((l) => l.name)]).toEqual([null, ['rokkit', 'zod']]);
  });

  it.skip('pending: L5 libraries — the read moves to the loader: a failed read is the error, not an empty index', async () => {
    const read = await readLibraries(api({ tryGetLibs: async () => down }));
    expect([read.error, read.libs]).toEqual(['Service Unavailable', []]);
  });
});
