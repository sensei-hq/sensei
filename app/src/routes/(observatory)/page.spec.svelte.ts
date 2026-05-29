// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// Synthesised PageData mirroring the loader's return shape.
type Data = {
  ftrDaily: Array<{ ftr_rate: number; day: string; session_count: number }>;
  projectFtrs: Array<{ id: string; name: string; kanji: string; ftr: number; sessions7d: number }>;
  teachings: Array<{ id: string; name: string; family?: string; instance_count: number }>;
  topRecommendations: Array<{ id: string; title: string; why: string; urgency: string; impact?: string }>;
  recentSessions: Array<{ id: string; task: string; project: string; startedAt: string; completedAt?: string; ftr?: number | null }>;
  sessionsTotal: number;
};

const emptyData = (): Data => ({
  ftrDaily: [],
  projectFtrs: [],
  teachings: [],
  topRecommendations: [],
  recentSessions: [],
  sessionsTotal: 0,
});

const matureData = (): Data => ({
  ftrDaily: [
    { ftr_rate: 0.72, day: '2026-05-15', session_count: 4 },
    { ftr_rate: 0.74, day: '2026-05-16', session_count: 6 },
    { ftr_rate: 0.78, day: '2026-05-17', session_count: 5 },
  ],
  projectFtrs: [
    { id: 'p1', name: 'lumen-auth',   kanji: '場', ftr: 0.78, sessions7d: 12 },
    { id: 'p2', name: 'lumen-canvas', kanji: '場', ftr: 0.82, sessions7d: 9  },
  ],
  teachings: [
    { id: 't1', name: 'Canvas smoothing pattern', family: 'pattern', instance_count: 4 },
  ],
  topRecommendations: [
    { id: 'r1', title: 'The AI does not know your auth.', why: 'Three sessions corrected refresh-or-device-flow this week.', urgency: 'high', impact: 'Projected FTR +14%' },
    { id: 'r2', title: 'Cache invalidation missed again.', why: '3rd time in lumen-cloud.', urgency: 'medium' },
  ],
  recentSessions: [
    { id: 's1', task: 'Fix auth', project: 'lumen-auth', startedAt: '2026-05-29T10:00:00Z', completedAt: '2026-05-29T10:38:00Z', ftr: 1 },
  ],
  sessionsTotal: 24,
});

describe('Observatory /+page — mode derivation', () => {
  it('renders early mode when no teachings and no recommendations exist', () => {
    const m = mountComponent(Page, { data: emptyData() });
    cleanup.push(m.destroy);
    const root = m.container.querySelector('[data-mode]');
    expect(root?.getAttribute('data-mode')).toBe('early');
  });

  it('renders mature mode once a recommendation exists', () => {
    const data = emptyData();
    data.topRecommendations = [{ id: 'r1', title: 'something', why: 'reason', urgency: 'high' }];
    const m = mountComponent(Page, { data });
    cleanup.push(m.destroy);
    const root = m.container.querySelector('[data-mode]');
    expect(root?.getAttribute('data-mode')).toBe('mature');
  });

  it('renders mature mode once a teaching exists', () => {
    const data = emptyData();
    data.teachings = [{ id: 't1', name: 'rule', family: 'pattern', instance_count: 1 }];
    const m = mountComponent(Page, { data });
    cleanup.push(m.destroy);
    const root = m.container.querySelector('[data-mode]');
    expect(root?.getAttribute('data-mode')).toBe('mature');
  });
});

describe('Observatory /+page — early mode rendering', () => {
  it('hides the FTR header in early mode', () => {
    const m = mountComponent(Page, { data: emptyData() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-ftr-header]')).toBeNull();
  });

  it('renders the 観 listening hero in early mode', () => {
    const m = mountComponent(Page, { data: emptyData() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-hero-early]')).not.toBeNull();
    expect(m.container.textContent).toContain('Still listening');
  });

  it('renders the placeholder insights (耳 / 試) in early mode', () => {
    const m = mountComponent(Page, { data: emptyData() });
    cleanup.push(m.destroy);
    const placeholder = m.container.querySelector('[data-early-insights]');
    expect(placeholder).not.toBeNull();
    expect(placeholder?.textContent).toContain('耳');
    expect(placeholder?.textContent).toContain('試');
  });

  it('renders a dynamic body referencing session count when sessions exist', () => {
    const data = emptyData();
    data.sessionsTotal = 4;
    data.projectFtrs = [
      { id: 'p1', name: 'lumen-auth', kanji: '場', ftr: 0, sessions7d: 4 },
    ];
    const m = mountComponent(Page, { data });
    cleanup.push(m.destroy);
    const hero = m.container.querySelector('[data-hero-early]');
    expect(hero?.textContent).toMatch(/4 sessions/);
  });
});

describe('Observatory /+page — mature mode rendering', () => {
  it('renders the FTR header in mature mode', () => {
    const m = mountComponent(Page, { data: matureData() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-ftr-header]')).not.toBeNull();
  });

  it('renders the top-recommendation hero (not the early hero)', () => {
    const m = mountComponent(Page, { data: matureData() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-hero-mature]')).not.toBeNull();
    expect(m.container.querySelector('[data-hero-early]')).toBeNull();
  });

  it('renders Recent Sessions in both modes', () => {
    const data = emptyData();
    data.recentSessions = [
      { id: 's1', task: 'Fix auth', project: 'lumen-auth', startedAt: '2026-05-29T10:00:00Z', completedAt: '2026-05-29T10:38:00Z', ftr: 1 },
    ];
    const m = mountComponent(Page, { data });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Recent sessions');
  });
});
