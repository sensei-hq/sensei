// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import TokensChart from './TokensChart.svelte';
import { mountComponent, type MountResult } from '$lib/test-mount.js';
import type { TurnDay } from '$lib/sessions-digest.js';

let m: MountResult | null = null;
afterEach(() => { m?.destroy(); m = null; });

const day = (d: string, tokens: number | null): TurnDay => ({
  day: d, label: d.slice(5), turns: 0, corrections: 0, ftr: null, tone: 'neutral',
  tokens, tokensIn: tokens, tokensOut: tokens === null ? null : 0,
});

describe('TokensChart', () => {
  // A quiet or new workspace: no tokens captured in the range. The axis ticks
  // collapsed onto one value and the chart threw (each_key_duplicate) — the e2e
  // Sessions digest found it on an empty workspace.
  it('draws its axis for a range with no tokens at all', () => {
    m = mountComponent(TokensChart, { days: [day('2026-10-28', null), day('2026-10-29', 0)] });
    expect(m.container.querySelector('[data-component="tokens-chart"]')).not.toBeNull();
  });
});
