// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import RecCardHarness from './RecCard.harness.svelte';
import type { RecCardVm } from './insights-board.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const rec = (over: Partial<RecCardVm> = {}): RecCardVm => ({
  id: 'r1',
  projectId: 'p1',
  projectName: 'sensei',
  projectKanji: '禅',
  title: 'Promote the retry pattern to a rule',
  why: 'seen in 8 places across the last week',
  impact: 'Projected FTR uplift toward 71%',
  column: 'now',
  ...over,
});

describe('RecCard', () => {
  it('renders title, why, impact and project name', () => {
    const m = mountComponent(RecCardHarness, { rec: rec() });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Promote the retry pattern to a rule');
    expect(m.container.textContent).toContain('seen in 8 places');
    expect(m.container.querySelector('[data-rec-impact]')?.textContent).toContain('Projected FTR uplift');
    expect(m.container.querySelector('[data-rec-project]')?.textContent).toContain('sensei');
  });

  it('offers the same three verbs on every card', () => {
    const m = mountComponent(RecCardHarness, { rec: rec() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-verb="apply"]')).not.toBeNull();
    expect(m.container.querySelector('[data-verb="review"]')).not.toBeNull();
    expect(m.container.querySelector('[data-verb="dismiss"]')).not.toBeNull();
  });

  it('marks Apply as the single recommended (highlighted) verb', () => {
    const m = mountComponent(RecCardHarness, { rec: rec() });
    cleanup.push(m.destroy);
    const apply = m.container.querySelector('[data-verb="apply"]');
    expect(apply?.textContent?.trim()).toBe('Apply');
    expect(apply?.hasAttribute('data-recommended')).toBe(true);
    // Highlighted = the rokkit primary (filled) variant; the secondary verbs are not.
    expect(apply?.getAttribute('data-variant')).toBe('primary');
    expect(m.container.querySelector('[data-verb="review"]')?.hasAttribute('data-recommended')).toBe(false);
    expect(m.container.querySelector('[data-verb="dismiss"]')?.hasAttribute('data-recommended')).toBe(false);
  });

  it('invokes the matching callback for each verb', () => {
    const onApply = vi.fn();
    const onReview = vi.fn();
    const onDismiss = vi.fn();
    const m = mountComponent(RecCardHarness, { rec: rec(), onApply, onReview, onDismiss });
    cleanup.push(m.destroy);

    (m.container.querySelector('[data-verb="apply"]') as HTMLElement).click();
    (m.container.querySelector('[data-verb="review"]') as HTMLElement).click();
    (m.container.querySelector('[data-verb="dismiss"]') as HTMLElement).click();

    expect(onApply).toHaveBeenCalledOnce();
    expect(onReview).toHaveBeenCalledOnce();
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it('shows the do-first marker only when focal', () => {
    const focalM = mountComponent(RecCardHarness, { rec: rec(), focal: true });
    cleanup.push(focalM.destroy);
    expect(focalM.container.querySelector('[data-do-first]')).not.toBeNull();

    const plainM = mountComponent(RecCardHarness, { rec: rec(), focal: false });
    cleanup.push(plainM.destroy);
    expect(plainM.container.querySelector('[data-do-first]')).toBeNull();
  });
});
