// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './SessionRow.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

type Props = Partial<{
  id: string;
  title: string;
  project: string;
  agent: string | null;
  outcome: string;
  ftr: boolean | null;
  corrections: number;
  quality: 'good' | 'bad' | 'ugly' | 'neutral';
  time: string;
  duration: string;
  folderRole: string | null;
  tokens: number | null;
  tokensIn: number | null;
  tokensOut: number | null;
  tokensLabel: string;
  onselect: (id: string) => void;
}>;

function mount(props: Props = {}) {
  const m = mountComponent(Harness, props);
  cleanup.push(m.destroy);
  return m;
}

const row = (m: { container: HTMLElement }): HTMLElement =>
  m.container.querySelector('[data-session-row]') as HTMLElement;

describe('SessionRow — quality dot', () => {
  for (const [quality, cls] of [
    ['good', 'bg-success'],
    ['bad', 'bg-warning'],
    ['ugly', 'bg-accent'],
    ['neutral', 'bg-ink-mute'],
  ] as const) {
    it(`colours the dot ${cls} for a ${quality} session`, () => {
      const m = mount({ quality });
      expect(row(m).getAttribute('data-quality')).toBe(quality);
      expect(row(m).querySelector('span.rounded-full')?.className).toContain(cls);
    });
  }
});

describe('SessionRow — FTR badge agrees with the ftr column', () => {
  it('shows a green ✓ FTR only when ftr is true', () => {
    const m = mount({ ftr: true });
    const badge = row(m).querySelector('.text-success');
    expect(badge?.textContent).toContain('✓ FTR');
  });

  it('shows an amber "no FTR" when ftr is explicitly false', () => {
    const m = mount({ ftr: false, quality: 'bad' });
    const text = row(m).textContent ?? '';
    expect(text).toContain('no FTR');
    expect(text).not.toContain('✓ FTR');
  });

  it('shows a faint dash when the session is not scored yet', () => {
    const m = mount({ ftr: null, quality: 'neutral' });
    // The FTR badge is the faint cell carrying the uppercase tracking treatment —
    // query it specifically so the (also-faint) token column doesn't shadow it.
    expect(row(m).querySelector('.text-ink-faint.uppercase')?.textContent).toContain('—');
  });
});

describe('SessionRow — meta', () => {
  it('renders time, duration, corrections label, project and agent', () => {
    const m = mount({ time: '09:14', duration: '42m', corrections: 0, project: 'sensei', agent: 'zed' });
    const text = row(m).textContent ?? '';
    expect(text).toContain('09:14');
    expect(text).toContain('42m');
    expect(text).toContain('first-try'); // 0 corrections
    expect(text).toContain('sensei');
    expect(text).toContain('zed');
  });

  it('labels non-zero corrections as "N× rework"', () => {
    const m = mount({ corrections: 3, ftr: false, quality: 'bad' });
    expect(row(m).textContent ?? '').toContain('3× rework');
  });

  it('falls back to "unlinked" for a project-less session', () => {
    const m = mount({ project: '' });
    expect(row(m).textContent ?? '').toContain('unlinked');
  });

  it('renders the token label with an in/out breakdown tooltip', () => {
    const m = mount({ tokensLabel: '7.7k', tokensIn: 6000, tokensOut: 1732 });
    const text = row(m).textContent ?? '';
    expect(text).toContain('7.7k');
    const cell = row(m).querySelector('[title*="in ·"]');
    expect(cell?.getAttribute('title')).toContain('6,000 in · 1,732 out');
  });
});

describe('SessionRow — folder-role chip (multi-repo)', () => {
  it('renders the folder-role chip when folderRole is populated', () => {
    const m = mount({ folderRole: 'backend' });
    const chip = row(m).querySelector('[data-folder-role]');
    expect(chip).toBeTruthy();
    expect(chip?.getAttribute('data-folder-role')).toBe('backend');
    expect(chip?.textContent).toContain('backend');
  });

  it('omits the chip when folderRole is absent (single-folder / observatory)', () => {
    const m = mount({ folderRole: null });
    expect(row(m).querySelector('[data-folder-role]')).toBeNull();
  });
});

describe('SessionRow — interaction', () => {
  it('calls onselect with the session id when clicked', () => {
    const onselect = vi.fn();
    const m = mount({ id: 's-42', onselect });
    row(m).click();
    expect(onselect).toHaveBeenCalledWith('s-42');
  });
});
