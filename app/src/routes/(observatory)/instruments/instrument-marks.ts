/**
 * The Instruments screen's marks, each with a kanji AND a Solar Bold Duotone
 * icon (#133), so the glyph preference can show either. Pure data, moved out
 * of +page.svelte so a spec can check every entry carries both.
 */

import type { InstrumentsTab } from './instruments-view';

/** A mark: the kanji, and the icon that stands for it in icon mode when there
 *  is one (none falls back to the kanji). */
export type Mark = { kanji: string; icon?: string };

/** The three tabs. The third is labelled Health (its id predates the label). */
export const INSTRUMENT_TABS: Array<{ id: InstrumentsTab; kanji: string; icon: string; label: string; hint: string }> = [
  { id: 'playground', kanji: '具', icon: 'i-solar-test-tube-bold-duotone', label: 'Playground', hint: 'what can these instruments do?' },
  { id: 'replay', kanji: '録', icon: 'i-solar-play-stream-bold-duotone', label: 'Replay', hint: 'what did the assistant do?' },
  { id: 'insights', kanji: '健', icon: 'i-solar-heart-pulse-bold-duotone', label: 'Health', hint: 'what should we change?' },
];

/** The kind filter chips (全 all, 作 actions, 問 queries), per the mockup. */
export const KIND_CHIPS: Array<{ id: 'all' | 'query' | 'action'; label: string; kanji: string; icon: string }> = [
  { id: 'all', label: 'All', kanji: '全', icon: 'i-solar-widget-4-bold-duotone' },
  { id: 'action', label: 'Actions', kanji: '作', icon: 'i-solar-bolt-bold-duotone' },
  { id: 'query', label: 'Queries', kanji: '問', icon: 'i-solar-magnifer-bold-duotone' },
];

/** A tool's kind mark; a tool of no known kind gets a dot and no icon. */
export function kindMark(kind: 'query' | 'action' | 'other'): Mark {
  const chip = KIND_CHIPS.find((c) => c.id === kind);
  return chip ? { kanji: chip.kanji, icon: chip.icon } : { kanji: '·' };
}
