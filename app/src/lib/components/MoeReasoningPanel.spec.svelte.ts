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
  modelsUsed: ['gemma4', 'opus-4-8'],
  suggestedRevision: null,
  ...over,
});

describe('MoeReasoningPanel', () => {
  it('renders headline, body and the real models used (no fabricated consensus/roles)', () => {
    const m = mountComponent(MoeReasoningPanelHarness, { reasoning: reasoning() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="impact-moe-headline"]')?.textContent)
      .toContain('FTR climbed');
    expect(m.container.querySelector('[data-testid="impact-moe-body"]')?.textContent)
      .toContain('corrected less');
    // No fabricated consensus vote tally is rendered.
    expect(m.container.querySelector('[data-testid="impact-moe-consensus"]')).toBeNull();
    // Real model names appear, as plain chips (no proposer/challenger role text).
    expect(m.container.querySelector('[data-testid="impact-moe-model-gemma4"]')).toBeTruthy();
    expect(m.container.querySelector('[data-testid="impact-moe-model-opus-4-8"]')).toBeTruthy();
    expect(m.container.querySelector('[data-testid="impact-moe-models"]')?.textContent)
      .not.toContain('proposer');
  });

  it('omits body / models / revision when the trace only carries a headline', () => {
    const m = mountComponent(MoeReasoningPanelHarness, {
      reasoning: reasoning({ body: null, modelsUsed: [], suggestedRevision: null }),
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="impact-moe-headline"]')).toBeTruthy();
    expect(m.container.querySelector('[data-testid="impact-moe-body"]')).toBeNull();
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
