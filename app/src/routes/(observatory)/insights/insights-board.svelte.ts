/**
 * Observatory · Insights (Learnings triage) — state.
 *
 * Owns the whole triage screen's runtime: the server board, the active
 * project-filter chip, optimistic card removals, and the Apply / Dismiss
 * actions. Components downstream are pure templates — every derivation and
 * every mutation lives here.
 *
 * Bucketing is trivial on purpose: the daemon assigns each item a `column`
 * and the client trusts it (the spec's wrong-gate is exactly a client that
 * re-buckets and disagrees with the server). We only filter by `column` and
 * shape the wire rows into view-models.
 *
 * The api client is passed into each async method as the seam, so the state
 * is testable with a hand-rolled mock and never reaches for a global.
 */

import { SvelteSet } from 'svelte/reactivity';
import type { SenseiApi } from '$lib/api.js';
import type {
  InsightsBoard,
  InsightColumn,
  InsightProjectRef,
  InsightRecommendation,
  InsightMemory,
  InsightPattern,
  InsightCorrection,
} from '$lib/types.js';

/**
 * The one recommended verb, identical on every rec card. This is the
 * flagship of the one-decision-one-default theme: Apply is highlighted;
 * Review and Dismiss are one keystroke away but visually secondary.
 */
export const RECOMMENDED_VERB = 'Apply' as const;

/** The all-quiet board — the api fallback and the initial value. */
export const EMPTY_INSIGHTS_BOARD: InsightsBoard = {
  counts: { now: 0, soon: 0, settled: 0 },
  projects: [],
  recommendations: [],
  memories: [],
  patterns: [],
  corrections: [],
};

// ── View-models ────────────────────────────────────────────────────────────
// Camel-cased, join-resolved shapes the cards render directly.

export interface RecCardVm {
  id: string;
  projectId: string;
  projectName: string;
  projectKanji: string;
  title: string;
  why: string;
  /** Plain-language impact sentence; '' when the wire sent none. */
  impact: string;
  column: InsightColumn;
}

export interface ViolationCardVm {
  id: string;
  projectId: string;
  title: string;
  content: string;
  violatedCount: number;
}

export interface CorrectionCardVm {
  id: string;
  text: string;
  count: number;
}

export interface PatternCardVm {
  id: string;
  name: string;
  family: string | null;
  instanceCount: number;
  lifecycle: string;
}

export interface MemoryRowVm {
  id: string;
  title: string;
  content: string;
  /** Strength as a 0–1 fraction (bar fill). */
  strength: number;
  scope: string;
  violatedCount: number;
}

export interface NowColumn {
  violations: ViolationCardVm[];
  recs: RecCardVm[];
  corrections: CorrectionCardVm[];
  count: number;
}

export interface SoonColumn {
  recs: RecCardVm[];
  patterns: PatternCardVm[];
  memories: MemoryRowVm[];
  count: number;
}

export interface SettledColumn {
  memories: MemoryRowVm[];
  patterns: PatternCardVm[];
  count: number;
}

// ── Pure mappers ────────────────────────────────────────────────────────────

/** Kanji for a rec's project, joined from the board's project list. */
function kanjiFor(projects: InsightProjectRef[], projectId: string): string {
  return projects.find((p) => p.id === projectId)?.kanji ?? '場';
}

export function toRecVm(r: InsightRecommendation, projects: InsightProjectRef[]): RecCardVm {
  return {
    id: r.id,
    projectId: r.project_id,
    projectName: r.name,
    projectKanji: kanjiFor(projects, r.project_id),
    title: r.title,
    why: r.why ?? '',
    impact: r.impact ?? '',
    column: r.column,
  };
}

export function toViolationVm(m: InsightMemory): ViolationCardVm {
  return {
    id: m.id,
    projectId: m.project_id,
    title: m.title,
    content: m.content,
    violatedCount: m.violated_count,
  };
}

export function toMemoryRowVm(m: InsightMemory): MemoryRowVm {
  return {
    id: m.id,
    title: m.title,
    content: m.content,
    // Clamp to 0–1 so the bar never overflows even if the daemon rescales.
    strength: Math.max(0, Math.min(1, m.strength)),
    scope: m.scope,
    violatedCount: m.violated_count,
  };
}

export function toPatternVm(p: InsightPattern): PatternCardVm {
  return {
    id: p.id,
    name: p.name,
    family: p.family,
    instanceCount: p.instance_count,
    lifecycle: p.lifecycle,
  };
}

export function toCorrectionVm(c: InsightCorrection): CorrectionCardVm {
  return { id: c.id, text: c.text, count: c.count };
}

// ── State ───────────────────────────────────────────────────────────────────

export class InsightsBoardState {
  board = $state<InsightsBoard>(EMPTY_INSIGHTS_BOARD);
  /** Active project-filter chip. null = all projects (the default). */
  selectedProjectId = $state<string | null>(null);
  /** Ids optimistically removed after Apply / Dismiss, before the round-trip.
   *  A SvelteSet so in-place add/delete/clear stay reactive. */
  removedIds = new SvelteSet<string>();
  /** Set when an action round-trip fails; the card is restored alongside. */
  actionError = $state<string | null>(null);
  /** True while a project-scope refetch is in flight. */
  loading = $state(false);

  constructor(board: InsightsBoard = EMPTY_INSIGHTS_BOARD) {
    this.board = board;
  }

  // Visible rows — everything the user hasn't just acted on. Only
  // recommendations are removable; memories/patterns/corrections are
  // display-only, so they pass through untouched.
  #visibleRecs(column: InsightColumn): RecCardVm[] {
    return this.board.recommendations
      .filter((r) => r.column === column && !this.removedIds.has(r.id))
      .map((r) => toRecVm(r, this.board.projects));
  }

  #memories(column: InsightColumn): InsightMemory[] {
    return this.board.memories.filter((m) => m.column === column);
  }

  get projects(): InsightProjectRef[] {
    return this.board.projects;
  }

  get now(): NowColumn {
    const violations = this.#memories('now').map(toViolationVm);
    const recs = this.#visibleRecs('now');
    const corrections = this.board.corrections
      .filter((c) => c.column === 'now')
      .map(toCorrectionVm);
    return { violations, recs, corrections, count: violations.length + recs.length + corrections.length };
  }

  get soon(): SoonColumn {
    const recs = this.#visibleRecs('soon');
    const patterns = this.board.patterns
      .filter((p) => p.column === 'soon')
      .map(toPatternVm);
    const memories = this.#memories('soon').map(toMemoryRowVm);
    return { recs, patterns, memories, count: recs.length + patterns.length + memories.length };
  }

  get settled(): SettledColumn {
    // In-force memories, strongest first — the "how we work here" shelf.
    const memories = this.#memories('settled')
      .map(toMemoryRowVm)
      .sort((a, b) => b.strength - a.strength);
    const patterns = this.board.patterns
      .filter((p) => p.column === 'settled')
      .map(toPatternVm);
    return { memories, patterns, count: memories.length + patterns.length };
  }

  /** Header micro-KPI + column counts, honest after optimistic removals. */
  get counts(): { now: number; soon: number; settled: number } {
    return { now: this.now.count, soon: this.soon.count, settled: this.settled.count };
  }

  /** True when every column is empty — drives the whole-screen quiet state. */
  get isEmpty(): boolean {
    return this.now.count + this.soon.count + this.settled.count === 0;
  }

  #remove(id: string): void {
    this.removedIds.add(id);
  }

  #restore(id: string): void {
    this.removedIds.delete(id);
  }

  /**
   * Re-scope every column to one project (or all). Refetches so the server
   * counts stay honest rather than filtering the cross-project board client-
   * side. Optimistic removals reset — the fresh board is the truth.
   */
  async selectProject(api: SenseiApi, projectId: string | null): Promise<void> {
    if (projectId === this.selectedProjectId) return;
    this.selectedProjectId = projectId;
    this.removedIds.clear();
    this.actionError = null;
    this.loading = true;
    this.board = await api.getInsights(projectId ?? undefined);
    this.loading = false;
  }

  /**
   * Apply a recommendation. The card is removed optimistically; the accept
   * call schedules the daemon's MeasureVerdicts follow-up so the before/after
   * FTR is captured. On failure the card is restored and the error surfaced —
   * never swallowed.
   */
  async apply(api: SenseiApi, rec: RecCardVm): Promise<void> {
    this.actionError = null;
    this.#remove(rec.id);
    const res = await api.acceptProjectRecommendation(rec.projectId, rec.id);
    if (!res.ok) {
      this.#restore(rec.id);
      this.actionError = `Couldn't apply "${rec.title}" — it may already be decided. Reloading may help.`;
    }
  }

  /** Dismiss a recommendation. Same optimistic-then-verify shape as apply. */
  async dismiss(api: SenseiApi, rec: RecCardVm): Promise<void> {
    this.actionError = null;
    this.#remove(rec.id);
    const res = await api.rejectProjectRecommendation(rec.projectId, rec.id);
    if (!res.ok) {
      this.#restore(rec.id);
      this.actionError = `Couldn't dismiss "${rec.title}". Reloading may help.`;
    }
  }
}
