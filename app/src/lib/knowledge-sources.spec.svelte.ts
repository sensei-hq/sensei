import { describe, it, expect, vi } from 'vitest';
import { validateSourceUrl, KnowledgeSourcesState } from './knowledge-sources.svelte.js';
import type { KnowledgeSource } from './types.js';
import type { SenseiApi } from './api.js';

describe('validateSourceUrl', () => {
  it('rejects an empty url', () => {
    expect(validateSourceUrl('')).toBe('url required');
  });

  it('rejects an unparseable url', () => {
    expect(validateSourceUrl('not a url')).toBe('invalid url');
  });

  it('rejects a non-loopback http url', () => {
    expect(validateSourceUrl('http://evil.com')).toBe('non-loopback url must be https');
  });

  it('accepts an https url', () => {
    expect(validateSourceUrl('https://hive.example.com')).toBeNull();
  });

  it('accepts http on 127.0.0.1', () => {
    expect(validateSourceUrl('http://127.0.0.1:8080')).toBeNull();
  });

  it('accepts http on localhost', () => {
    expect(validateSourceUrl('http://localhost')).toBeNull();
  });
});

// Hand-rolled mock api — only the four knowledge-source methods the state
// touches are stubbed; cast to SenseiApi since the state only uses these.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    listKnowledgeSources: vi.fn().mockResolvedValue({ sources: [] }),
    createKnowledgeSource: vi.fn().mockResolvedValue({ ok: true, data: {} }),
    deleteKnowledgeSource: vi.fn().mockResolvedValue({ ok: true, data: undefined }),
    syncKnowledgeSource: vi.fn().mockResolvedValue({ ok: true, data: {} }),
    ...overrides,
  } as unknown as SenseiApi;
}

const sourceFixture = (over: Partial<KnowledgeSource> = {}): KnowledgeSource => ({
  id: 's1',
  kind: 'hive',
  name: 'Hive',
  url: 'https://hive.example.com',
  namespace_id: null,
  direction: 'pull',
  last_seq: 12,
  enabled: true,
  ...over,
});

describe('KnowledgeSourcesState', () => {
  it('load() populates sources and clears loading', async () => {
    const api = mockApi({
      listKnowledgeSources: vi.fn().mockResolvedValue({ sources: [sourceFixture()] }),
    });
    const state = new KnowledgeSourcesState();
    await state.load(api);
    expect(state.sources).toHaveLength(1);
    expect(state.sources[0].id).toBe('s1');
    expect(state.loading).toBe(false);
  });

  it('add() with an invalid url sets addError and does NOT call createKnowledgeSource', async () => {
    const create = vi.fn();
    const api = mockApi({ createKnowledgeSource: create });
    const state = new KnowledgeSourcesState();
    state.url = 'http://evil.com';
    await state.add(api);
    expect(state.addError).toBe('non-loopback url must be https');
    expect(create).not.toHaveBeenCalled();
  });

  it('add() with a valid url calls createKnowledgeSource and reloads', async () => {
    const create = vi.fn().mockResolvedValue({ ok: true, data: sourceFixture() });
    const list = vi.fn().mockResolvedValue({ sources: [sourceFixture()] });
    const api = mockApi({ createKnowledgeSource: create, listKnowledgeSources: list });
    const state = new KnowledgeSourcesState();
    state.url = 'https://hive.example.com';
    state.key = 'secret';
    state.name = 'Hive';
    await state.add(api);
    expect(create).toHaveBeenCalledWith({ url: 'https://hive.example.com', key: 'secret', name: 'Hive' });
    expect(list).toHaveBeenCalled();
    expect(state.url).toBe('');
    expect(state.addError).toBeNull();
  });

  it('add() keeps form + sets addError when create fails', async () => {
    const create = vi.fn().mockResolvedValue({ ok: false, error: { status: 400, message: 'bad' } });
    const api = mockApi({ createKnowledgeSource: create });
    const state = new KnowledgeSourcesState();
    state.url = 'https://hive.example.com';
    await state.add(api);
    expect(state.addError).toBe('bad');
    expect(state.url).toBe('https://hive.example.com');
  });
});
