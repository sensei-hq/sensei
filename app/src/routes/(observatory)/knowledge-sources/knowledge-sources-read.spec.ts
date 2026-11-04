// L5: the Knowledge sources read moves out of the component's onMount and into
// the route loader. The loader is a read like any other, so the #288 rule holds
// there too: a list that could not be read is the error it was, never an empty
// list that the screen would render as "No knowledge sources."
// The read itself lives in `knowledge-sources-load.ts` (the libraries
// precedent), and the state's load reuses it — one copy, not two.
import { describe, it, expect } from 'vitest';
import type { KnowledgeSource } from '$lib/types.js';
import {
  readKnowledgeSources,
  type KnowledgeSourcesListApi,
} from './knowledge-sources-load.js';

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

describe('readKnowledgeSources — what the Knowledge sources route load returns', () => {
  it('the read moves to the loader: returns every source', async () => {
    const read = await readKnowledgeSources(api());
    expect([read.error, read.sources.map((s) => s.id)]).toEqual([null, ['s1', 's2']]);
  });

  it('the read moves to the loader: a failed read is the error, not an empty list', async () => {
    const read = await readKnowledgeSources(api({ tryListKnowledgeSources: async () => down }));
    expect([read.error, read.sources]).toEqual(['Service Unavailable', []]);
  });
});
