import { describe, it, expect } from 'vitest';
import { componentStatusView } from './health-status';
import type { ComponentStatus } from './health-types';

// Audit item 11 (2026-10-28): a component's status was mapped to a colour three
// times — the disc, the label, the gate numeral — and they disagreed on one row
// (checking: accent label beside an ink-soft numeral). One view decides.
describe('componentStatusView', () => {
  it('gives each status one colour, one label, one glyph', () => {
    expect(componentStatusView('ready')).toEqual({ label: null, text: 'text-success', border: 'border-success', glyph: 'check', dim: false });
    expect(componentStatusView('checking')).toEqual({ label: 'checking', text: 'text-accent', border: 'border-accent', glyph: 'spinner', dim: false });
    expect(componentStatusView('installing')).toEqual({ label: 'installing', text: 'text-accent', border: 'border-accent', glyph: 'spinner', dim: false });
    expect(componentStatusView('failed')).toEqual({ label: 'blocked', text: 'text-accent', border: 'border-accent', glyph: 'mark', dim: false });
    expect(componentStatusView('pending')).toEqual({ label: null, text: 'text-ink-faint', border: 'border-ink-faint', glyph: null, dim: true });
  });

  it('says what an install is doing when the daemon names it', () => {
    expect(componentStatusView('installing', 'configuring').label).toBe('configuring');
    expect(componentStatusView('ready', 'configuring').label).toBeNull();
  });

  it('the numeral, label and disc of one row always share a colour', () => {
    for (const s of ['pending', 'checking', 'installing', 'ready', 'failed'] as ComponentStatus[]) {
      const v = componentStatusView(s);
      expect(v.border.replace('border-', 'text-'), s).toBe(v.text);
    }
  });
});
