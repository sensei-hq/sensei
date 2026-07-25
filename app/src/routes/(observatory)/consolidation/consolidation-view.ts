// Governance Tier-2 consolidation — pure data-shaping for the review screen.
//
// Tier-1 gathers a scope's raw rules; Tier-2 asks a model to merge them into one
// coherent, deduped ruleset; a human approves (or leaves it unapproved =
// keep-separate). This module owns the wire->view mapping + the before->after
// diff stats so the component stays a pure template and the logic is
// unit-testable without mounting anything. No runes here.

/** The consolidated-ruleset row as returned by
 *  `GET /api/knowledge/rules/consolidated`. `null` on the wire when no
 *  consolidation has ever run for the scope. `conflicts` is an opaque JSON array
 *  (empty today; reserved for per-rule merge notes). */
export interface ConsolidatedRuleset {
  id: string;
  version: number;
  /** The merged ruleset as markdown — the "after". */
  content: string;
  conflicts: unknown[];
  model: string | null;
  /** proposed | approved | superseded. */
  status: string;
}

/** Result of `POST /api/knowledge/rules/consolidate`: either a fresh proposed
 *  version, or a skip (no rules / unchanged since last merge). */
export type ConsolidateResult =
  | { skipped: true; reason: string }
  | { skipped?: false; id: string; version: number; status: string; model: string | null; content: string };

/** Semantic tone for the current status — drives the named-token colour. */
export type ConsolidationTone = 'accent' | 'success' | 'ink';

export interface StatusMeta {
  glyph: string;
  tone: ConsolidationTone;
  label: string;
  /** True only for a proposed version — the one the Approve gate acts on. */
  approvable: boolean;
}

/** Status → glyph + tone + label. The lookup IS the rendering rule, so it lives
 *  with the domain, not in the component. Unknown/absent degrades to a muted
 *  "unknown" rather than throwing. */
export function statusMeta(status: string | null | undefined): StatusMeta {
  switch (status) {
    case 'proposed':
      return { glyph: '新', tone: 'accent', label: 'proposed merge', approvable: true };
    case 'approved':
      return { glyph: '結', tone: 'success', label: 'approved', approvable: false };
    case 'superseded':
      return { glyph: '別', tone: 'ink', label: 'superseded', approvable: false };
    default:
      return { glyph: '·', tone: 'ink', label: status ?? 'unknown', approvable: false };
  }
}

/** Named-token utility class for a tone. `ink` → muted ink (no status colour). */
export function toneClass(tone: ConsolidationTone, kind: 'text' | 'bg' | 'border' = 'text'): string {
  if (tone === 'success') return `${kind}-success`;
  if (tone === 'accent') return `${kind}-accent`;
  return `${kind}-ink-mute`;
}

/** One rendered section of the merged markdown, split on top-level `#`/`##`
 *  headings so the "after" reads as a structured ruleset rather than a wall of
 *  text. A section with no heading keeps an empty `heading`. */
export interface RuleSection {
  heading: string;
  body: string;
}

/** Split merged markdown into heading-led sections. Content before the first
 *  heading becomes a leading section with an empty heading (preamble). Pure. */
export function splitSections(markdown: string): RuleSection[] {
  const lines = (markdown ?? '').replace(/\r\n/g, '\n').split('\n');
  const sections: RuleSection[] = [];
  let heading = '';
  let body: string[] = [];
  const flush = () => {
    const text = body.join('\n').trim();
    if (heading || text) sections.push({ heading, body: text });
  };
  for (const line of lines) {
    const m = /^#{1,3}\s+(.*)$/.exec(line);
    if (m) {
      flush();
      heading = m[1].trim();
      body = [];
    } else {
      body.push(line);
    }
  }
  flush();
  return sections;
}

/** A cell in the before->after diff strip. `delta` is a signed display string
 *  (or null when the stat has no meaningful delta). */
export interface DiffStat {
  label: string;
  before: string;
  after: string;
  delta: string | null;
  tone: ConsolidationTone;
}

/**
 * Before->after stats for the diff strip. `sourceCount` is the number of raw
 * Tier-1 rules that fed the merge (from `GET /api/knowledge/rules` when the
 * caller could resolve it; `null` when unknown — then the "rules on disk" row is
 * omitted). The merged ruleset is always one artifact, so the reduction is
 * `1 - sourceCount`. Pure.
 */
export function diffStats(ruleset: ConsolidatedRuleset, sourceCount: number | null): DiffStat[] {
  const stats: DiffStat[] = [];
  if (sourceCount != null && sourceCount > 0) {
    const reduction = 1 - sourceCount;
    stats.push({
      label: 'Rules on disk',
      before: String(sourceCount),
      after: '1',
      delta: reduction === 0 ? '±0' : `${reduction}`,
      tone: reduction < 0 ? 'accent' : 'ink',
    });
  }
  stats.push({
    label: 'Sections',
    before: '—',
    after: String(splitSections(ruleset.content).filter((s) => s.heading).length),
    delta: null,
    tone: 'ink',
  });
  stats.push({
    label: 'Conflicts',
    before: '—',
    after: String(ruleset.conflicts?.length ?? 0),
    delta: null,
    tone: (ruleset.conflicts?.length ?? 0) > 0 ? 'accent' : 'ink',
  });
  stats.push({
    label: 'Version',
    before: '—',
    after: `v${ruleset.version}`,
    delta: null,
    tone: 'ink',
  });
  return stats;
}
