// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import RecentSessions from './RecentSessions.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

type Session = {
  id: string;
  task: string;
  project: string;
  startedAt: string;
  completedAt?: string;
  ftr?: number | null;
};

const session = (over: Partial<Session> = {}): Session => ({
  id: 's-1', task: 'Fix refresh token rotation', project: 'lumen-auth',
  startedAt: '2026-05-29T10:42:00Z', completedAt: '2026-05-29T11:20:00Z',
  ftr: 1, ...over,
});

describe('RecentSessions', () => {
  it('renders the "Recent sessions" header', () => {
    const m = mountComponent(RecentSessions, { sessions: [session()] });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Recent sessions');
  });

  it('renders one row per session with task + project visible', () => {
    const sessions: Session[] = [
      session({ id: 's-1', task: 'Fix refresh token rotation', project: 'lumen-auth' }),
      session({ id: 's-2', task: 'Bezier smoothing tool',      project: 'lumen-canvas' }),
    ];
    const m = mountComponent(RecentSessions, { sessions });
    cleanup.push(m.destroy);
    const rows = m.container.querySelectorAll('[data-session-row]');
    expect(rows).toHaveLength(2);
    expect(m.container.textContent).toContain('Fix refresh token rotation');
    expect(m.container.textContent).toContain('lumen-auth');
    expect(m.container.textContent).toContain('lumen-canvas');
  });

  it('marks a successful (ftr >= 1) session with a success indicator', () => {
    const m = mountComponent(RecentSessions, {
      sessions: [session({ id: 's-1', ftr: 1 })],
    });
    cleanup.push(m.destroy);
    const dot = m.container.querySelector('[data-session-row="s-1"] [data-ftr-dot]');
    expect(dot?.getAttribute('data-tone')).toBe('success');
  });

  it('marks a failed (ftr < 1) session with a warning indicator', () => {
    const m = mountComponent(RecentSessions, {
      sessions: [session({ id: 's-1', ftr: 0 })],
    });
    cleanup.push(m.destroy);
    const dot = m.container.querySelector('[data-session-row="s-1"] [data-ftr-dot]');
    expect(dot?.getAttribute('data-tone')).toBe('warn');
  });

  it('renders nothing when sessions is empty', () => {
    const m = mountComponent(RecentSessions, { sessions: [] });
    cleanup.push(m.destroy);
    // No header, no rows — section is collapsed
    expect(m.container.querySelectorAll('[data-session-row]')).toHaveLength(0);
    expect(m.container.textContent).not.toContain('Recent sessions');
  });

  it('renders the duration in mm format when completedAt is present', () => {
    const m = mountComponent(RecentSessions, {
      sessions: [session({
        id: 's-1',
        startedAt:   '2026-05-29T10:00:00Z',
        completedAt: '2026-05-29T10:38:00Z',
      })],
    });
    cleanup.push(m.destroy);
    const row = m.container.querySelector('[data-session-row="s-1"]');
    expect(row?.textContent).toContain('38m');
  });

  it('omits the duration when completedAt is missing', () => {
    const m = mountComponent(RecentSessions, {
      sessions: [session({ id: 's-1', completedAt: undefined })],
    });
    cleanup.push(m.destroy);
    const dur = m.container.querySelector('[data-session-row="s-1"] [data-duration]');
    expect(dur).toBeNull();
  });
});
