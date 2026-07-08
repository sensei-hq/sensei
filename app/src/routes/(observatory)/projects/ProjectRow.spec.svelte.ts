// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './ProjectRow.harness.svelte';

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
  last: boolean;
  onOpen: (id: string, name: string) => void;
}>;

function mount(props: Props = {}) {
  const m = mountComponent(Harness, props);
  cleanup.push(m.destroy);
  return m;
}

const row = (m: { container: HTMLElement }): HTMLElement =>
  m.container.querySelector('[data-project-row]') as HTMLElement;

describe('ProjectRow — active variant', () => {
  it('carries data-status="active" and shows the ftr signal', () => {
    const m = mount({ sessions7d: 4, ftr14d: 0.88 });
    expect(row(m).getAttribute('data-status')).toBe('active');
    expect(row(m).textContent ?? '').toContain('88% ftr');
  });

  it('shows the repos · libs cell', () => {
    const m = mount({ sessions7d: 4, repos_count: 3, libs_count: 9 });
    const text = row(m).textContent ?? '';
    expect(text).toContain('3 repos');
    expect(text).toContain('9 libs');
  });
});

describe('ProjectRow — dormant variant', () => {
  it('carries data-status="dormant" and shows a last-session signal, not ftr', () => {
    const m = mount({ sessions7d: 0, last_session_at: null });
    expect(row(m).getAttribute('data-status')).toBe('dormant');
    const text = row(m).textContent ?? '';
    expect(text).toContain('last ·');
    expect(text).not.toContain('ftr');
  });

  it('shows a dormant status pill', () => {
    const m = mount({ sessions7d: 0 });
    const pill = m.container.querySelector('[data-component="proj-pill"][data-tone="dormant"]');
    expect(pill?.textContent).toBe('dormant');
  });
});

describe('ProjectRow — archived variant', () => {
  it('dims the archived row', () => {
    const m = mount({ maturity: 'archived', sessions7d: 0 });
    expect(row(m).getAttribute('data-status')).toBe('archived');
    expect(row(m).className).toMatch(/opacity-60/);
  });
});

describe('ProjectRow — vision truncation', () => {
  it('renders the vision on one truncated line (never wraps)', () => {
    const m = mount({ vision: 'a long vision that must never wrap to a second row' });
    const vision = row(m).querySelector('[data-vision]') as HTMLElement;
    expect(vision).toBeTruthy();
    expect(vision.className).toMatch(/\btruncate\b/);
  });

  it('omits the vision line when empty', () => {
    const m = mount({ vision: null });
    expect(row(m).querySelector('[data-vision]')).toBeNull();
  });
});

describe('ProjectRow — separator', () => {
  it('draws a bottom hairline for a non-last row', () => {
    const m = mount({ last: false });
    expect(row(m).className).toMatch(/\bborder-b\b/);
  });

  it('draws no bottom hairline for the last row', () => {
    const m = mount({ last: true });
    expect(row(m).className).not.toMatch(/\bborder-b\b/);
  });
});

describe('ProjectRow — interaction', () => {
  it('calls onOpen with the project id and name when clicked', () => {
    const onOpen = vi.fn();
    const m = mount({ name: 'rokkit', onOpen });
    row(m).click();
    expect(onOpen).toHaveBeenCalledWith('p-1', 'rokkit');
  });
});
