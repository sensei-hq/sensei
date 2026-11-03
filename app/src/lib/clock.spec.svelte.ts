import { describe, it, expect, vi } from 'vitest';
import { Clock } from './clock.svelte.js';

// Audit item 14 (2026-10-28): Activity Logs read Date.now() once per data load,
// so "running", "next in …" and "x ago" went stale while the screen sat open.
describe('Clock', () => {
  it('moves while a view needs it, and stops when none does', async () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(1_000_000);
      const clock = new Clock(1000);
      clock.start();
      clock.start(); // a second view
      vi.setSystemTime(1_005_000);
      await vi.advanceTimersByTimeAsync(1000);
      expect(clock.now).toBe(1_006_000);
      clock.stop();
      vi.setSystemTime(1_010_000);
      await vi.advanceTimersByTimeAsync(1000);
      expect(clock.now).toBe(1_011_000); // one view still needs it
      clock.stop();
      const frozen = clock.now;
      await vi.advanceTimersByTimeAsync(5000);
      expect(clock.now).toBe(frozen);
    } finally {
      vi.useRealTimers();
    }
  });
});
