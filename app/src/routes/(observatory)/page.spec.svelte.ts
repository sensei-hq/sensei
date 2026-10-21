// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { ObservatoryToday, ObservatoryFtr } from '$lib/types.js';
import type { RecentSession } from './recent-sessions.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

type Data = {
  today: ObservatoryToday | null;
  todayError: string | null;
  ftr: ObservatoryFtr | null;
  ftrError: string | null;
  recentSessions: RecentSession[];
};

const ftr = (over: Partial<ObservatoryFtr> = {}): ObservatoryFtr => ({
  ftr14d: 0.5,
  ftr14dPrev: 0.7333333333333333,
  ftrTrend: [0.2, 0.4, 0.6, 0.5, 0.7, 0.5, 0.55, 0.6, 0.5, 0.45, 0.5, 0.55, 0.5, 0.5],
  sessions7d: 10,
  ...over,
});

// The daemon owns the maturity gate and every user-facing string. These
// fixtures mirror the two variants the /api/observatory/today handler emits.
const matureToday = (): ObservatoryToday => ({
  greeting: 'Good evening',
  today: 'Tue · 7 Jul',
  dataMaturity: 'mature',
  hero: {
    kanji: '聴',
    koan: 'Recurring corrections in DemoBot',
    body: '4 corrective prompts clustered here — a guard or skill could prevent the repeat.',
    impact: null,
    action: 'Review recommendation',
    source: '',
    noticed: '',
  },
  insights: [
    { kanji: '繰', label: 'Recommendation', text: 'Recurring corrections in rokkit', tag: 'high', tone: 'warn' },
    { kanji: '昇', label: 'Teaching adopted', text: 'Canvas smoothing promoted to rule', tag: '+7% FTR', tone: 'good' },
  ],
  adopted: [
    { when: '1w ago', what: 'Prefer queue_duration over queue_time_ms', scope: 'project', source: 'from 3 prompts' },
  ],
  recentSessions: [],
});

const earlyToday = (): ObservatoryToday => ({
  greeting: 'Good morning',
  today: 'Tue · 7 Jul',
  dataMaturity: 'early',
  hero: {
    kanji: '観',
    koan: 'Still listening.',
    body: 'sensei has watched 4 sessions so far. Nothing confident enough to teach yet.',
    impact: '~2 more sessions until first lesson',
    action: null,
    source: '',
    noticed: 'since setup · 2d ago',
  },
  insights: [
    { kanji: '耳', label: 'Listening', text: 'Watching prompt style. Early signal forming.', tag: 'forming', tone: 'mute' },
  ],
  adopted: [],
  recentSessions: [],
});

const recent = (): RecentSession[] => [
  { id: 's1', task: 'Fix auth', project: 'lumen-auth', startedAt: '2026-07-06T10:00:00Z', completedAt: '2026-07-06T10:38:00Z', ftr: 1, corrections: 0 },
];

describe('Observatory /+page — maturity is the daemon decision', () => {
  it('renders the mode the daemon reports (mature)', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-mode]')?.getAttribute('data-mode')).toBe('mature');
  });

  it('renders the mode the daemon reports (early)', () => {
    const m = mountComponent(Page, { data: { today: earlyToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-mode]')?.getAttribute('data-mode')).toBe('early');
  });
});

describe('Observatory /+page — FTR header', () => {
  it('shows the chip integer and a directional arrow in mature mode', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    const header = m.container.querySelector('[data-ftr-header]');
    expect(header?.textContent).toContain('50');
    const arrow = m.container.querySelector('[data-ftr-arrow]');
    expect(arrow?.getAttribute('data-ftr-arrow')).toBe('down');
    expect(arrow?.textContent).toContain('↓');
    expect(arrow?.textContent).toContain('23%');
  });

  it('is present but dimmed in early mode', () => {
    const m = mountComponent(Page, { data: { today: earlyToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    const header = m.container.querySelector('[data-ftr-header]');
    expect(header).not.toBeNull();
    expect(header?.getAttribute('data-dim')).toBe('true');
  });

  it('is not dimmed in mature mode', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-ftr-header]')?.hasAttribute('data-dim')).toBe(false);
  });
});

describe('Observatory /+page — hero koan', () => {
  it('renders the daemon koan, not a lorem placeholder', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    const hero = m.container.querySelector('[data-component="hero-koan"]');
    expect(hero?.textContent).toContain('Recurring corrections in DemoBot');
  });

  it('renders the action button only when the hero carries an action', () => {
    const mature = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(mature.destroy);
    expect(mature.container.querySelector('[data-hero-action]')).not.toBeNull();

    const early = mountComponent(Page, { data: { today: earlyToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(early.destroy);
    expect(early.container.querySelector('[data-hero-action]')).toBeNull();
  });

  it('renders no dangling provenance when hero.source is empty', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    // matureToday has empty source AND empty noticed → no provenance node at all.
    expect(m.container.querySelector('[data-hero-source]')).toBeNull();
  });
});

describe('Observatory /+page — insights + adopted', () => {
  it('renders one insight card per daemon insight, titled for the mode', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    expect(m.container.querySelectorAll('[data-component="insight-card"]')).toHaveLength(2);
    expect(m.container.textContent).toContain('Also worth noticing');
  });

  it('titles the insights column "Early signals" in early mode', () => {
    const m = mountComponent(Page, { data: { today: earlyToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Early signals');
  });

  it('renders adopted cards from the daemon adopted lane', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    expect(m.container.querySelectorAll('[data-component="adopted-card"]')).toHaveLength(1);
    expect(m.container.textContent).toContain('Prefer queue_duration over queue_time_ms');
  });

  it('shows the adopted empty-state (never a "loading" placeholder) when the lane is empty', () => {
    const m = mountComponent(Page, { data: { today: earlyToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    const empty = m.container.querySelector('[data-adopted-empty]');
    expect(empty).not.toBeNull();
    expect(empty?.textContent).toContain('No teachings adopted yet');
  });

  it('shows the insights empty-state when there are no insights', () => {
    const today = matureToday();
    today.insights = [];
    const m = mountComponent(Page, { data: { today, todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-insights-empty]')).not.toBeNull();
  });
});

describe('Observatory /+page — voice + recent sessions', () => {
  it('keeps all copy in the lowercase "sensei" voice', () => {
    const early = mountComponent(Page, { data: { today: earlyToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(early.destroy);
    expect(early.container.textContent).not.toContain('Sensei');

    const mature = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: [] } as Data });
    cleanup.push(mature.destroy);
    expect(mature.container.textContent).not.toContain('Sensei');
  });

  it('renders the recent-sessions list from the shaped rows', () => {
    const m = mountComponent(Page, { data: { today: matureToday(), todayError: null, ftr: ftr(), ftrError: null, recentSessions: recent() } as Data });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Recent sessions');
    expect(m.container.querySelector('[data-session-row="s1"]')).not.toBeNull();
  });
});

// #288: a read that failed is shown as a failure, never as the early state or
// a quiet "—".
describe('Observatory /+page — a failed read', () => {
  it('shows an error with retry when Today could not be read', () => {
    const m = mountComponent(Page, {
      data: { today: null, todayError: 'Service Unavailable', ftr: ftr(), ftrError: null, recentSessions: [] } as Data,
    });
    cleanup.push(m.destroy);
    const state = m.container.querySelector('[data-screen-state="error"]');
    expect(state?.textContent).toContain('Service Unavailable');
    expect(m.container.querySelector('[data-mode]')).toBeNull();
    expect(m.container.textContent).not.toContain('Still listening');
  });

  it('says the FTR could not be read, distinct from no data yet', () => {
    const m = mountComponent(Page, {
      data: { today: matureToday(), todayError: null, ftr: null, ftrError: 'Bad Gateway', recentSessions: [] } as Data,
    });
    cleanup.push(m.destroy);
    const err = m.container.querySelector('[data-ftr-error]');
    expect(err?.textContent).toContain('unavailable');
    expect(err?.getAttribute('title')).toBe('Bad Gateway');
    expect(m.container.querySelector('[data-mode]')?.getAttribute('data-mode')).toBe('mature');
  });
});
