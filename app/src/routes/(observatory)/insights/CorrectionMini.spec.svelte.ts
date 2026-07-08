// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import CorrectionMiniHarness from './CorrectionMini.harness.svelte';
import type { CorrectionCardVm } from './insights-board.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const correction = (over: Partial<CorrectionCardVm> = {}): CorrectionCardVm => ({
  id: 'c1',
  text: 'stop guessing test commands, read the manifest',
  count: 4,
  ...over,
});

describe('CorrectionMini', () => {
  it('renders the correction text and its count', () => {
    const m = mountComponent(CorrectionMiniHarness, { correction: correction() });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('stop guessing test commands');
    expect(m.container.querySelector('[data-correction-count]')?.textContent).toContain('4');
  });
});
