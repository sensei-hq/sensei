import { describe, it, expect, vi } from 'vitest';
import { ConsolidationState, type ConsolidationApi } from './consolidation-state.svelte';

const ok = <T>(data: T) => ({ ok: true as const, data });
const fail = (status: number, message: string) => ({ ok: false as const, error: { status, message } });

function api(over: Partial<ConsolidationApi> = {}): ConsolidationApi {
  return {
    consolidateRules: vi.fn().mockResolvedValue(ok({ id: 'r1', status: 'proposed' })),
    approveConsolidatedRuleset: vi.fn().mockResolvedValue(ok({ id: 'r1', status: 'approved', scope: 'global' })),
    ...over,
  } as ConsolidationApi;
}

describe('ConsolidationState', () => {
  it('consolidates and reloads', async () => {
    const s = new ConsolidationState();
    const reload = vi.fn().mockResolvedValue(undefined);
    await s.consolidate(api(), reload);
    expect(reload).toHaveBeenCalledOnce();
    expect(s.busy).toBeNull();
    expect(s.notice).toBeNull();
  });

  it('says why the daemon skipped, without reloading', async () => {
    const s = new ConsolidationState();
    const reload = vi.fn();
    await s.consolidate(api({ consolidateRules: vi.fn().mockResolvedValue(ok({ skipped: true, reason: 'nothing overlaps' })) }), reload);
    expect(s.notice).toBe('nothing overlaps');
    expect(reload).not.toHaveBeenCalled();
  });

  it('says why a consolidation failed', async () => {
    const s = new ConsolidationState();
    await s.consolidate(api({ consolidateRules: vi.fn().mockResolvedValue(fail(503, 'Service Unavailable')) }), vi.fn());
    expect(s.notice).toContain('Service Unavailable');
    expect(s.busy).toBeNull();
  });

  it('approves a ruleset and reloads; a refusal says why', async () => {
    const s = new ConsolidationState();
    const a = api();
    const reload = vi.fn().mockResolvedValue(undefined);
    await s.approve(a, 'r1', reload);
    expect(a.approveConsolidatedRuleset).toHaveBeenCalledWith('r1');
    expect(reload).toHaveBeenCalledOnce();

    await s.approve(api({ approveConsolidatedRuleset: vi.fn().mockResolvedValue(fail(409, 'Conflict')) }), 'r1', reload);
    expect(s.notice).toContain('Conflict');
    expect(reload).toHaveBeenCalledOnce();
  });

  it('marks which action is running', async () => {
    const s = new ConsolidationState();
    let release!: () => void;
    const pending = new Promise<void>((r) => (release = r));
    const run = s.approve(api({ approveConsolidatedRuleset: vi.fn().mockImplementation(async () => { await pending; return ok({}); }) }), 'r1', vi.fn());
    expect(s.busy).toBe('approve');
    release();
    await run;
    expect(s.busy).toBeNull();
  });
});
