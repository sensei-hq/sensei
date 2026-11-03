// L5: the Knowledge sources read moves out of the component's onMount and into
// the route loader. The loader is a read like any other, so the #288 rule holds
// there too: a list that could not be read is the error it was, never an empty
// list that the screen would render as "No knowledge sources."
// The read itself lives in `knowledge-sources-load.ts` (the libraries
// precedent), and the state's load reuses it — one copy, not two.
import { describe, it, expect } from 'vitest';
import type { SenseiApi } from '$lib/api.js';
import type { KnowledgeSource } from '$lib/types.js';

type KnowledgeSourcesRead = { sources: KnowledgeSource[]; error: string | null };
type KnowledgeSourcesListApi = Pick<SenseiApi, 'tryListKnowledgeSources'>;

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const source = (id: string, over: Partial<KnowledgeSource> = {}): KnowledgeSource => ({
  id,
  kind: 'hive',
  name: `Hive ${id}`,
  url: `https://${id}.example.com`,
  namespace_id: null,
  direction: 'pull',
  last_seq: 12,
  enabled: true,
  ...over,
});

const api = (over: Partial<KnowledgeSourcesListApi> = {}): KnowledgeSourcesListApi => ({
  tryListKnowledgeSources: async () => ok({ sources: [source('s1'), source('s2')] }),
  ...over,
});

// Phase A guard: `knowledge-sources-load.ts` does not exist yet, so the test
// fails on the missing behaviour instead of a module-resolution error that
// would fail the whole file before any assertion ran. The path is a variable
// so the typechecker does not resolve it either. Phase B makes this a static
// import.
const LOADER = './knowledge-sources-load.js';

async function readKnowledgeSources(api: KnowledgeSourcesListApi): Promise<KnowledgeSourcesRead> {
  const mod = (await import(/* @vite-ignore */ LOADER).catch(() => null)) as unknown as {
    readKnowledgeSources?: (a: KnowledgeSourcesListApi) => Promise<KnowledgeSourcesRead>;
  } | null;
  if (!mod?.readKnowledgeSources) throw new Error('knowledge-sources-load.ts has no readKnowledgeSources (the read has not moved to the loader yet)');
  return mod.readKnowledgeSources(api);
}

describe('readKnowledgeSources — what the Knowledge sources route load returns', () => {
  it.skip('pending: L5 knowledge-sources — the read moves to the loader: returns every source', async () => {
    const read = await readKnowledgeSources(api());
    expect([read.error, read.sources.map((s) => s.id)]).toEqual([null, ['s1', 's2']]);
  });

  it.skip('pending: L5 knowledge-sources — the read moves to the loader: a failed read is the error, not an empty list', async () => {
    const read = await readKnowledgeSources(api({ tryListKnowledgeSources: async () => down }));
    expect([read.error, read.sources]).toEqual(['Service Unavailable', []]);
  });
});
