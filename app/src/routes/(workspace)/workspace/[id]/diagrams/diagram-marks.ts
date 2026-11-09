import { KANJI_ICONS } from '$lib/glyph-vocabulary';

// The mark each stat card shows in kanji mode (D87 §9, N5a).
//
// rokkit's stat card draws `icon` only in icon mode and `mark` in kanji mode —
// one rule everywhere (#133): kanji mode shows kanji, icon mode icons. The old
// header broke it by always drawing the Solar icon, so the migration's cards
// went blank in the default (kanji) mode. Each count therefore gets the
// vocabulary's kanji for what it counts, keyed by the card's label; the card's
// own `icon` still draws in icon mode, and `KANJI_ICONS` is what makes the two
// modes name the same thing.
//
// Every kanji here is in the vocabulary already — a spec pins that — so no new
// vocabulary entry is invented for a count.
const MARKS: Record<string, string> = {
  // The workspace's numbers (packages/codemap's workspaceStats), plus Repos.
  Modules: '構', // composition — the Structure diagram's own mark
  Repos: '場', // a place with files — the icon this card already carries
  Symbols: '型', // a type or a form
  Edges: '縁', // a connection between two things
  Upward: '先', // ahead — an edge that points up the layering
  'Skip a layer': '層', // a layer, skipped
  Cycles: '環', // a cycle — the Diagrams list's own mark for the Cycles view
  'Same layer · down': '流', // flow within the layer
  // The design's counts (rokkit's overviewCounts). `源` for the schema keeps `表`
  // for the tables themselves; the ER diagram's own mark is `表`.
  Schema: '源', // a source — the database the objects come from
  Table: '表', // a table, as Database · ER's own mark says
  View: '観', // to observe — a view is a lens over other data
  Enum: '規', // a fixed rule set — a closed list of labels
  Reference: '鍵', // a key — what a reference points at
  'Materialized view': '層', // stored, hence layers — the icon this count carries
  Function: '作', // to make — it returns a value
  Procedure: '手', // a hand — invoked for its effects
  Trigger: '鈴', // a bell — it fires on an event
};

/** The kanji a count's card shows; `件` (a count of things) for one not named here. */
export function statMark(label: string): string {
  // rokkit's counts are plural from two up (`Tables`), singular at one (`Table`).
  const mark = MARKS[label] ?? MARKS[label.replace(/s$/, '')];
  return mark && KANJI_ICONS[mark] ? mark : '件';
}
