// Observatory · Instruments — the screen's pure view logic: which tab a deep
// link opens, each tab's hero copy, search over the
// playground rail, and payload formatting. No state, no reads.
import type { PlaygroundGroup } from './playground-groups';

export type InstrumentsTab = 'playground' | 'replay' | 'insights';

/** The tab a `?tab=` deep link opens (the Sessions digest links to Replay). */
export function initialTab(param: string | null): InstrumentsTab {
  return param === 'replay' || param === 'insights' ? param : 'playground';
}

/** Per-tab hero copy, phrased as in docs/mockups/Sensei/lib/instruments.jsx. */
export function heroCopy(tab: InstrumentsTab): { tagline: string; sub: string } {
  if (tab === 'replay') {
    return {
      tagline: 'Every instrument call, in order.',
      sub: 'Step through the tools the assistant reached for during a session. Pure request + response — what was asked, what came back, how long it took.',
    };
  }
  if (tab === 'insights') {
    return {
      tagline: 'Toolset health.',
      sub: 'How your instruments are performing — usage, effectiveness signals, and where the assistant is under- or over-reaching.',
    };
  }
  return {
    tagline: 'Sensei · what your instruments can do.',
    sub: "Every MCP server your assistants are configured with, sensei's own first. Each tool runs through its server, so what you see is what an assistant gets.",
  };
}

/** The rail narrowed to tools whose name or summary matches; empty groups go. */
export function filterGroups(groups: PlaygroundGroup[], query: string): PlaygroundGroup[] {
  const ql = query.trim().toLowerCase();
  if (!ql) return groups;
  return groups
    .map((g) => ({
      ...g,
      tools: g.tools.filter((t) => t.name.toLowerCase().includes(ql) || t.summary.toLowerCase().includes(ql)),
    }))
    .filter((g) => g.tools.length > 0);
}

/** While searching every group with a hit is open; otherwise the user's choice. */
export function collapsedFor(
  shown: PlaygroundGroup[],
  chosen: Record<string, boolean>,
  query: string,
): Record<string, boolean> {
  if (!query.trim()) return chosen;
  return Object.fromEntries(shown.map((g) => [g.id, false]));
}

/** A request/response payload: pretty JSON for an object, the raw string otherwise. */
export function fmtPayload(value: unknown): string {
  if (value == null) return '—';
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function fmtDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

/** A tool's kind, as every part of the Instruments screen shows it (audit
 *  2026-10-28, item 8: coloured three times in the markup). */
export interface ToolKindView {
  label: string;
  /** The Kanji `tone` prop. */
  tone: 'accent' | 'success' | 'muted';
  text: string;
  /** The left-edge colour. */
  edge: string;
  hint: string;
  /** The execute button's label. */
  runLabel: string;
}

export function toolKindView(kind: 'action' | 'query' | 'other' | string): ToolKindView {
  if (kind === 'action') return { label: 'action', tone: 'accent', text: 'text-accent', edge: 'var(--accent)', hint: 'performs an operation', runLabel: 'Run →' };
  if (kind === 'query') return { label: 'query', tone: 'success', text: 'text-success', edge: 'var(--success)', hint: 'returns information', runLabel: 'Query →' };
  return { label: 'call', tone: 'muted', text: 'text-ink-soft', edge: 'var(--ink-soft)', hint: 'returns information', runLabel: 'Query →' };
}

/** A recorded call's verdict (#90): used, partial, ignored, else failed or
 *  unclassified — one glyph and colour (it was decided three times). */
export function callVerdictView(call: { verdict: 'used' | 'partial' | 'ignored' | null; success?: boolean | null }): {
  glyph: string;
  text: string;
  edge: string;
} {
  if (call.verdict === 'used') return { glyph: '用', text: 'text-success', edge: 'var(--success)' };
  if (call.verdict === 'partial') return { glyph: '部', text: 'text-warning', edge: 'var(--warning)' };
  if (call.verdict === 'ignored') return { glyph: '無', text: 'text-ink-soft', edge: 'var(--ink-soft)' };
  return { glyph: call.success === false ? '✗' : '問', text: 'text-accent', edge: 'var(--accent)' };
}

