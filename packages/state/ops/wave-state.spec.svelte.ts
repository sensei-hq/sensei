import { describe, it, expect } from 'vitest';
import { WaveState } from './wave-state.svelte.js';
import type { StreamFrame } from '../events.js';

// Wave rows shared by the jobs and sync states: totals open a wave, the
// terminal closes it with settled counts. Mid-wave settled is unknown by
// design — only the terminal names it — so the row never invents one.
describe('WaveState', () => {
  const frame = (
    wave: string,
    key: string,
    data: Record<string, unknown>,
    done = false,
  ): StreamFrame => ({
    v: 1,
    id: 1,
    ts: '2026-11-21T10:00:00.000Z',
    entity: wave,
    action: done ? 'update' : 'add',
    key,
    data,
    ...(done ? { done: true as const } : {}),
  });

  it('totals open a wave; the terminal closes it with counts', () => {
    const st = new WaveState(['transcript', 'metric']);
    st.apply(frame('transcript', '9', { total: 2 }));
    expect(st.items).toEqual([{ wave: 'transcript', key: '9', total: 2, settled: 0, done: false }]);
    st.apply(frame('transcript', '9', { settled: 2, total: 2 }, true));
    expect(st.items).toEqual([{ wave: 'transcript', key: '9', total: 2, settled: 2, done: true }]);
  });

  it('foreign entities and malformed totals never create rows', () => {
    const st = new WaveState(['sync']);
    st.apply(frame('transcript', '9', { total: 2 }));
    st.apply(frame('sync', 'p1', { nope: 1 }));
    expect(st.items).toEqual([]);
  });

  it('a terminal for an unseen wave still records it done', () => {
    const st = new WaveState(['sync']);
    st.apply(frame('sync', 'p1', { settled: 3, total: 3 }, true));
    expect(st.items).toEqual([{ wave: 'sync', key: 'p1', total: 3, settled: 3, done: true }]);
  });
});
