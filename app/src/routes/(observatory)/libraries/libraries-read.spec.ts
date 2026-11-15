// L5: the Libraries read moves out of the component's onMount and into the
// route loader. The loader is a read like any other, so the #288 rule holds
// there too: a list that could not be read is the error it was, never an empty
// index that the screen would render as "No libraries indexed."
// The read itself lives in `libraries-load.ts` (the workspaces precedent), and
// the state's load/retry reuses it — one copy, not two.
import { describe, it, expect } from 'vitest';
import type { LibEntry } from '$lib/types.js';
import { readLibraries } from './libraries-load.js';
import type { LibrariesApi } from './libraries-state.svelte.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const lib = (id: string, name = id) => ({ id, name, repoCount: 1, repos: [] }) as unknown as LibEntry;

/** The arguments every `tryGetLibs` was asked with, so #63 stays pinned. */
const asked: Array<Parameters<LibrariesApi['tryGetLibs']>[0]> = [];

const api = (over: Partial<LibrariesApi> = {}): LibrariesApi => ({
  tryGetLibs: async (params) => {
    asked.push(params);
    return ok({ total: 2, libs: [lib('a', 'rokkit'), lib('b', 'zod')] });
  },
  tryGetLibraryUsage: async () => ok({ usage: [] }),
  tryGetLibrarySkills: async () => ok([]),
  tryGetLibraryAgents: async () => ok([]),
  tryGetLibraryDocs: async () => ok([]),
  ...over,
});

describe('readLibraries — what the Libraries route load returns', () => {
  it('returns every detected library', async () => {
    const read = await readLibraries(api());
    expect([read.error, read.libs.map((l) => l.name)]).toEqual([null, ['rokkit', 'zod']]);
  });

  it('asks for every library, never only the shared ones (#63)', async () => {
    asked.length = 0;
    await readLibraries(api());
    expect(asked[0]?.shared).toBeFalsy();
  });

  it('a failed read is the error, not an empty index', async () => {
    const read = await readLibraries(api({ tryGetLibs: async () => down }));
    expect([read.error, read.libs]).toEqual(['Service Unavailable', []]);
  });
});
