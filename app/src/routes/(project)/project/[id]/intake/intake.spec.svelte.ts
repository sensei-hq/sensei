import { describe, it, expect } from 'vitest';
import { IntakeState, type IntakeApi } from './intake.svelte.js';
import type { IntakeGuide, PlaybookRecommendation } from '$lib/types.js';

const GUIDE: IntakeGuide = {
  frame: 'Describe the chunk of work.',
  axes: [{ kind: 'axis', axis: 'intent', prompt: 'What are you doing?', help: null }],
  playbooks: [
    { name: 'debug_flow', title: 'Debug Flow', when_to_use: '', opening_tone: '', method_ref: null },
    { name: 'gsd', title: 'Get Stuff Done', when_to_use: '', opening_tone: '', method_ref: null },
  ],
};

const rec = (over: Partial<PlaybookRecommendation> = {}): PlaybookRecommendation => ({
  playbook: 'debug_flow', rationale: 'a fix', lifecycle: 'stable', intent: 'bug', risk: 'low',
  rule: 'r', defaulted: false, opening_tone: 'careful', when_to_use: '',
  auto_select: false, trust: { n: 0, ftr: 0 }, ...over,
});

// Records every recommendPlaybook body so tests can assert the confirm payload.
function fakeApi(result: PlaybookRecommendation, opts: { ok?: boolean } = {}): IntakeApi & { calls: Record<string, unknown>[] } {
  const calls: Record<string, unknown>[] = [];
  return {
    calls,
    getIntakeGuide: async () => GUIDE,
    recommendPlaybook: async (body) => {
      calls.push(body);
      return opts.ok === false
        ? { ok: false, error: { status: 500, message: 'boom' } }
        : { ok: true, data: result };
    },
  };
}

describe('IntakeState', () => {
  it('resolves the playbook title from the guide catalog', () => {
    const s = new IntakeState(GUIDE, 'proj-1');
    s.rec = rec({ playbook: 'gsd' });
    expect(s.playbookTitle).toBe('Get Stuff Done');
  });

  it('falls back to the raw name when the catalog lacks the playbook', () => {
    const s = new IntakeState(GUIDE, 'proj-1');
    s.rec = rec({ playbook: 'unknown_pb' });
    expect(s.playbookTitle).toBe('unknown_pb');
  });

  it('preview previews without confirming, then confirm records', async () => {
    const s = new IntakeState(GUIDE, 'proj-1');
    s.chunk = 'fix the crash';
    const api = fakeApi(rec());
    await s.recommend(api);
    expect(s.phase).toBe('recommended');
    expect(s.rec?.playbook).toBe('debug_flow');
    // First call is the preview leg — scoped to the project (a run is never global).
    expect(api.calls[0]).toEqual({ chunk: 'fix the crash', preview: true, project_id: 'proj-1' });

    await s.confirm(api);
    expect(s.phase).toBe('recorded');
    // Confirm reuses the classified axes (no re-classify) with confirm:true, still project-scoped.
    expect(api.calls[1]).toEqual({ lifecycle: 'stable', intent: 'bug', risk: 'low', confirm: true, project_id: 'proj-1' });
  });

  it('auto-selects (auto-confirms) when the daemon says trusted', async () => {
    const s = new IntakeState(GUIDE, 'proj-1');
    s.chunk = 'tweak a low-risk thing';
    const api = fakeApi(rec({ auto_select: true, trust: { n: 12, ftr: 0.9 } }));
    await s.recommend(api);
    // Preview then an automatic confirm — no manual confirm() call.
    expect(api.calls.length).toBe(2);
    expect(api.calls[1]).toEqual({ lifecycle: 'stable', intent: 'bug', risk: 'low', confirm: true, project_id: 'proj-1' });
    expect(s.phase).toBe('recorded');
  });

  it('ignores an empty chunk', async () => {
    const s = new IntakeState(GUIDE, 'proj-1');
    s.chunk = '   ';
    const api = fakeApi(rec());
    await s.recommend(api);
    expect(api.calls.length).toBe(0);
    expect(s.phase).toBe('describe');
  });

  it('surfaces a recommend error', async () => {
    const s = new IntakeState(GUIDE, 'proj-1');
    s.chunk = 'fix the crash';
    const api = fakeApi(rec(), { ok: false });
    await s.recommend(api);
    expect(s.phase).toBe('error');
    expect(s.error).toBe('boom');
  });

  it('reset returns to the describe phase', async () => {
    const s = new IntakeState(GUIDE, 'proj-1');
    s.chunk = 'x'; s.rec = rec(); s.phase = 'recorded';
    s.reset();
    expect(s.phase).toBe('describe');
    expect(s.chunk).toBe('');
    expect(s.rec).toBeNull();
  });
});
