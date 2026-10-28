import { describe, it, expect, vi } from 'vitest';
import { assignChainRoles, ChainRoles } from './chain-roles.svelte.js';

// Audit item 13 (2026-10-28): Settings · Inference held its whole slice — load,
// per-role save status, per-member busy, "saved" timers — in the component, and
// its clear-then-set role write repeated the wizard's commit step.

const ok = <T>(data: T) => ({ ok: true as const, data });
const refused = (message: string) => ({ ok: false as const, error: { message } });

const chain = (id: string, over: Record<string, unknown> = {}) => ({
  id, name: id, capability: 'reasoning', role: null, description: null, maxFallbackAttempts: 2, isActive: true, models: [], ...over,
});

describe('assignChainRoles — the one role write', () => {
  it('clears every old chain before setting any new one, and skips what did not change', async () => {
    const calls: string[] = [];
    const api = { setGatewayChainRole: vi.fn(async (id: string, role: string | null) => { calls.push(`${id}=${role}`); return ok({}); }) };
    const refusals = await assignChainRoles(api as never, [
      { role: 'inference', prev: 'a', next: 'b' },
      { role: 'embedding', prev: null, next: 'c' },
      { role: 'voice', prev: 'd', next: 'd' },
    ]);
    expect(calls).toEqual(['a=null', 'b=inference', 'c=embedding']);
    expect(refusals).toEqual([]);
  });

  it('collects what the daemon refused', async () => {
    const api = { setGatewayChainRole: vi.fn(async (id: string) => (id === 'b' ? refused('in use') : ok({}))) };
    expect(await assignChainRoles(api as never, [{ role: 'inference', prev: 'a', next: 'b' }])).toEqual(['assigning inference: in use']);
  });
});

describe('ChainRoles — Settings · Inference', () => {
  function harness(chains: unknown[], api: Record<string, unknown> = {}) {
    const full = {
      tryListGatewayChains: vi.fn(async () => ok({ chains })),
      setGatewayChainRole: vi.fn(async () => ok({})),
      moveGatewayChainModel: vi.fn(async () => ok({})),
      removeGatewayChainModel: vi.fn(async () => ok({})),
      ...api,
    };
    return { state: new ChainRoles(() => full as never), api: full };
  }

  it('loads the chains and who holds each role; a failed read is said', async () => {
    const h = harness([chain('a', { role: 'inference' }), chain('b')]);
    await h.state.load();
    expect(h.state).toMatchObject({ loading: false, loadError: null });
    expect(h.state.assignments.inference).toBe('a');
    const bad = new ChainRoles(() => ({ tryListGatewayChains: async () => refused('down') }) as never);
    await bad.load();
    expect(bad.loadError).toMatch(/down/);
  });

  it('offers a role only the active chains that can serve it', async () => {
    const h = harness([chain('a'), chain('off', { isActive: false }), chain('img', { capability: 'image' }), chain('taken', { role: 'voice' })]);
    await h.state.load();
    expect(h.state.optionsFor('inference').map((c) => c.id)).toEqual(['a']);
  });

  it('picks a chain for a role with the one role write, and says it saved', async () => {
    vi.useFakeTimers();
    try {
      const h = harness([chain('a', { role: 'inference' }), chain('b')]);
      await h.state.load();
      await h.state.pick('inference', 'b');
      expect(h.api.setGatewayChainRole).toHaveBeenNthCalledWith(1, 'a', null);
      expect(h.api.setGatewayChainRole).toHaveBeenNthCalledWith(2, 'b', 'inference');
      expect(h.state.assignments.inference).toBe('b');
      expect(h.state.saveStatus.inference).toBe('saved');
      await vi.advanceTimersByTimeAsync(1500);
      expect(h.state.saveStatus.inference).toBe('idle');
    } finally {
      vi.useRealTimers();
    }
  });

  it('a refused pick says why and re-reads the daemon', async () => {
    const h = harness([chain('a', { role: 'inference' }), chain('b')], {
      setGatewayChainRole: vi.fn(async (id: string) => (id === 'b' ? refused('taken') : ok({}))),
    });
    await h.state.load();
    await h.state.pick('inference', 'b');
    expect(h.state.saveStatus.inference).toBe('error');
    expect(h.state.saveError.inference).toMatch(/taken/);
    expect(h.api.tryListGatewayChains).toHaveBeenCalledTimes(2);
  });

  it('marks only the member being moved busy', async () => {
    let release: () => void = () => {};
    const gate = new Promise<void>((r) => (release = r));
    const h = harness([chain('a', { role: 'inference' })], {
      moveGatewayChainModel: vi.fn(async () => { await gate; return ok({}); }),
    });
    await h.state.load();
    const move = h.state.moveMember('inference', 'm1', -1);
    expect(h.state.memberBusy).toEqual({ m1: 'up' });
    release();
    await move;
    expect(h.state.memberBusy.m1).toBeNull();
  });
});
