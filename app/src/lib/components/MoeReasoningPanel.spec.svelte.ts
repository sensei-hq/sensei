// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import MoeReasoningPanelHarness from './MoeReasoningPanel.harness.svelte';
import type { ImpactReasoning } from '$lib/impact.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const reasoning = (over: Partial<ImpactReasoning> = {}): ImpactReasoning => ({
  headline: 'FTR climbed after the agent shipped',
  body: 'Sessions after acceptance corrected less and finished faster.',
  consensus: '3 positive · 0 neutral · 0 negative',
  models: [
    { name: 'gemma4', role: 'proposer', note: 'Contributed to the original consensus panel.' },
    { name: 'opus-4-8', role: 'reviewer', note: 'Confirmed the sustained lift.' },
  ],
  suggestedRevision: null,
  ...over,
});

describe('MoeReasoningPanel', () => {
  it('renders headline, body, consensus and every model note', () => {
    const m = mountComponent(MoeReasoningPanelHarness, { reasoning: reasoning() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="impact-moe-headline"]')?.textContent)
      .toContain('FTR climbed');
    expect(m.container.querySelector('[data-testid="impact-moe-body"]')?.textContent)
      .toContain('corrected less');
    expect(m.container.querySelector('[data-testid="impact-moe-consensus"]')?.textContent)
      .toContain('3 positive');
    expect(m.container.querySelector('[data-testid="impact-moe-model-gemma4"]')).toBeTruthy();
    expect(m.container.querySelector('[data-testid="impact-moe-model-opus-4-8"]')).toBeTruthy();
  });

  it('omits body / consensus / revision when the trace only carries a headline', () => {
    const m = mountComponent(MoeReasoningPanelHarness, {
      reasoning: reasoning({ body: null, consensus: null, models: [], suggestedRevision: null }),
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="impact-moe-headline"]')).toBeTruthy();
    expect(m.container.querySelector('[data-testid="impact-moe-body"]')).toBeNull();
    expect(m.container.querySelector('[data-testid="impact-moe-consensus"]')).toBeNull();
    expect(m.container.querySelector('[data-testid="impact-moe-models"]')).toBeNull();
    expect(m.container.querySelector('[data-testid="impact-moe-revision"]')).toBeNull();
  });

  it('shows the suggested revision only when present', () => {
    const m = mountComponent(MoeReasoningPanelHarness, {
      reasoning: reasoning({ suggestedRevision: 'Scope the agent to the core crate only.' }),
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="impact-moe-revision"]')?.textContent)
      .toContain('core crate');
  });

  it('paints the left edge with the verdict tone', () => {
    const warn = mountComponent(MoeReasoningPanelHarness, { reasoning: reasoning(), tone: 'warning' });
    cleanup.push(warn.destroy);
    expect(warn.container.querySelector('[data-testid="impact-moe-panel"]')?.className)
      .toMatch(/border-l-warning/);

    const good = mountComponent(MoeReasoningPanelHarness, { reasoning: reasoning(), tone: 'success' });
    cleanup.push(good.destroy);
    expect(good.container.querySelector('[data-testid="impact-moe-panel"]')?.className)
      .toMatch(/border-l-success/);
  });
});
