import { describe, it, expect, vi } from 'vitest';
import { UpgradeRun } from './upgrade-state.svelte.js';

// Audit item 13 (2026-10-28): the Upgrade page held its step machine, the Tauri
// listener and timers in the component — and two faults with them: a failed
// run_upgrade_steps was marked done with nothing failed (so it read "Up to date"
// and moved on), and in a browser it faked both steps succeeding.

function harness() {
  let emit: (p: { step: string; status: string; error?: string }) => void = () => {};
  const unlisten = vi.fn();
  const finish = vi.fn();
  const deps = {
    listen: vi.fn(async (handler: typeof emit) => { emit = handler; return unlisten; }),
    invoke: vi.fn(async () => {}),
    finish,
  };
  return { run: new UpgradeRun(), deps, emit: (p: Parameters<typeof emit>[0]) => emit(p), unlisten, finish };
}

describe('UpgradeRun', () => {
  it('follows each step and says which is running', async () => {
    const h = harness();
    await h.run.start(h.deps);
    h.emit({ step: 'prereqs', status: 'running' });
    expect(h.run.phase).toBe('running');
    expect(h.run.cardText).toBe('Prerequisite upgrades… (1/2)');
    expect(h.run.rows[0]).toMatchObject({ status: 'running', dot: 'busy', statusClass: 'text-accent', dim: false });
    expect(h.run.rows[1]).toMatchObject({ status: 'pending', dot: 'idle', dim: true });
    h.emit({ step: 'prereqs', status: 'done' });
    expect(h.run.progress).toBe('50%');
  });

  it('a clean run is up to date, and continues after a moment', async () => {
    vi.useFakeTimers();
    try {
      const h = harness();
      await h.run.start(h.deps);
      h.emit({ step: 'prereqs', status: 'done' });
      h.emit({ step: 'db_deploy', status: 'done' });
      h.emit({ step: 'complete', status: 'done' });
      expect(h.run).toMatchObject({ phase: 'ok', indicator: 'check' });
      expect(h.run.title).toEqual({ lead: 'Up to date.', tail: 'Resuming.', tailClass: 'text-success' });
      await vi.advanceTimersByTimeAsync(1200);
      expect(h.finish).toHaveBeenCalledTimes(1);
      expect(h.unlisten).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it('a failed step is said, with its reason', async () => {
    const h = harness();
    await h.run.start(h.deps);
    h.emit({ step: 'db_deploy', status: 'failed', error: 'dbd: relation exists' });
    h.emit({ step: 'complete', status: 'done' });
    expect(h.run.phase).toBe('partial');
    expect(h.run.rows[1]).toMatchObject({ status: 'failed', dot: 'fail', error: 'dbd: relation exists' });
  });

  it('a run that could not start is a failure, never "up to date"', async () => {
    const h = harness();
    h.deps.invoke.mockRejectedValueOnce(new Error('command not found'));
    await h.run.start(h.deps);
    expect(h.run.phase).toBe('partial');
    expect(h.run.error).toMatch(/command not found/);
    expect(h.run.title.lead).not.toMatch(/Up to date/);
  });

  it('outside the app it says it cannot run — no simulated progress', async () => {
    const run = new UpgradeRun();
    await run.start(null);
    expect(run.phase).toBe('unavailable');
    expect(run.rows.every((r) => r.status === 'pending')).toBe(true);
    expect(run.blurb).toMatch(/only in the sensei app/);
  });
});
