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

export function fmtDuration(ms: number | null): string {
  if (ms == null) return '—';
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

export function fmtDate(iso: string): string {
  return new Date(iso).toLocaleString();
}
