// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './ProjectCard.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

type Props = Partial<{
  name: string;
  client: string | null;
  vision: string | null;
  maturity: string;
  ftr14d: number;
  sessions7d: number;
  warn: boolean;
  repos_count: number;
  libs_count: number;
  last_session_at: string | null;
  icon: { kind?: string; value?: string } | null;
  onOpen: (id: string, name: string) => void;
}>;

function mount(props: Props = {}) {
  const m = mountComponent(Harness, props);
  cleanup.push(m.destroy);
  return m;
}

const card = (m: { container: HTMLElement }): HTMLElement =>
  m.container.querySelector('[data-project-card]') as HTMLElement;
const clientPill = (m: { container: HTMLElement }) =>
  m.container.querySelector('[data-component="proj-pill"]:not([data-tone])');
const statusPill = (m: { container: HTMLElement }) =>
  m.container.querySelector('[data-component="proj-pill"][data-tone="dormant"]');

describe('ProjectCard — active variant', () => {
  it('carries data-status="active"', () => {
    const m = mount({ sessions7d: 4 });
    expect(card(m).getAttribute('data-status')).toBe('active');
  });

  it('shows the ftr / repos / libs strip with real values', () => {
    const m = mount({ sessions7d: 4, ftr14d: 0.92, repos_count: 3, libs_count: 12 });
    const text = card(m).textContent ?? '';
    expect(text).toContain('ftr');
    expect(text).toContain('repos');
    expect(text).toContain('libs');
    expect(text).toContain('92');
    expect(text).toContain('3');
    expect(text).toContain('12');
  });

  it('does not show a 7d stat (dropped from the active card)', () => {
    const m = mount({ sessions7d: 4 });
    expect(card(m).textContent ?? '').not.toContain('7d');
  });

  it('renders no status pill for an active project', () => {
    const m = mount({ sessions7d: 4 });
    expect(statusPill(m)).toBeNull();
  });

  it('tints the ftr stat with the warning tone when warned', () => {
    const m = mount({ sessions7d: 4, warn: true });
    const warned = card(m).querySelector('.text-warning');
    expect(warned).toBeTruthy();
  });
});

describe('ProjectCard — dormant variant', () => {
  it('carries data-status="dormant" and a dormant status pill', () => {
    const m = mount({ sessions7d: 0 });
    expect(card(m).getAttribute('data-status')).toBe('dormant');
    expect(statusPill(m)?.textContent).toBe('dormant');
  });

  it('shows repos / libs / last session instead of ftr', () => {
    const m = mount({ sessions7d: 0, repos_count: 2, libs_count: 5, last_session_at: null });
    const text = card(m).textContent ?? '';
    expect(text).toContain('repos');
    expect(text).toContain('libs');
    expect(text).toContain('last session');
    expect(text).not.toContain('ftr');
  });
});

describe('ProjectCard — archived variant', () => {
  it('carries data-status="archived", dims the card and shows an archived pill', () => {
    const m = mount({ maturity: 'archived', sessions7d: 0 });
    expect(card(m).getAttribute('data-status')).toBe('archived');
    expect(card(m).className).toMatch(/opacity-60/);
    const pill = m.container.querySelector('[data-component="proj-pill"][data-tone="dormant"]');
    expect(pill?.textContent).toBe('archived');
  });
});

describe('ProjectCard — vision row', () => {
  it('renders the vision row when a vision is present', () => {
    const m = mount({ vision: 'a self-improving coding companion' });
    const vision = card(m).querySelector('[data-vision]');
    expect(vision?.textContent).toContain('self-improving');
  });

  it('omits the vision row entirely when the vision is empty', () => {
    const m = mount({ vision: null });
    expect(card(m).querySelector('[data-vision]')).toBeNull();
  });
});

describe('ProjectCard — client pill', () => {
  it('renders a client pill when the project has a client', () => {
    const m = mount({ client: 'acme' });
    expect(clientPill(m)?.textContent).toBe('acme');
  });

  it('omits the client pill when the client is null', () => {
    const m = mount({ client: null });
    expect(clientPill(m)).toBeNull();
  });
});

describe('ProjectCard — icon', () => {
  it('renders an image and no kanji glyph for an image icon', () => {
    const m = mount({ icon: { kind: 'image', value: 'http://x/y.png' } });
    expect(card(m).querySelector('img')).toBeTruthy();
    expect(card(m).querySelector('span.kanji')).toBeNull();
  });

  it('falls back to the 場 kanji glyph and no image when no icon is set', () => {
    const m = mount({ icon: null });
    const glyph = card(m).querySelector('span.kanji');
    expect(glyph?.textContent).toBe('場');
    expect(card(m).querySelector('img')).toBeNull();
  });
});

describe('ProjectCard — interaction', () => {
  it('calls onOpen with the project id and name when clicked', () => {
    const onOpen = vi.fn();
    const m = mount({ name: 'sensei', onOpen });
    card(m).click();
    expect(onOpen).toHaveBeenCalledWith('p-1', 'sensei');
  });
});
