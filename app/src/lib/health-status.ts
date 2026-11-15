// A setup/health component's status, as every part of a row shows it — the disc,
// its label and the gate numeral (audit 2026-10-28, item 11: three components
// mapped status to colour separately and disagreed on one row).
import type { ComponentStatus } from './health-types';

export interface ComponentStatusView {
  /** The mono label beside the disc — null when the disc alone says it
   *  (ready, pending), per the splash mockup. */
  label: string | null;
  /** The status colour, for text (the label, the numeral). */
  text: string;
  /** The same colour, for the disc's ring. */
  border: string;
  /** What the disc draws inside. */
  glyph: 'check' | 'spinner' | 'mark' | null;
  /** A row not yet reached is dimmed. */
  dim: boolean;
}

const VIEWS: Record<ComponentStatus, ComponentStatusView> = {
  ready: { label: null, text: 'text-success', border: 'border-success', glyph: 'check', dim: false },
  checking: { label: 'checking', text: 'text-accent', border: 'border-accent', glyph: 'spinner', dim: false },
  installing: { label: 'installing', text: 'text-accent', border: 'border-accent', glyph: 'spinner', dim: false },
  // Accent, as the mockup's "blocked": it needs you.
  failed: { label: 'blocked', text: 'text-accent', border: 'border-accent', glyph: 'mark', dim: false },
  pending: { label: null, text: 'text-ink-faint', border: 'border-ink-faint', glyph: null, dim: true }
};

/** The view for a status; an install the daemon names ("configuring") says so. */
export function componentStatusView(status: ComponentStatus, installingVerb?: string | null): ComponentStatusView {
  const view = VIEWS[status] ?? VIEWS.pending;
  return status === 'installing' && installingVerb ? { ...view, label: installingVerb } : view;
}
