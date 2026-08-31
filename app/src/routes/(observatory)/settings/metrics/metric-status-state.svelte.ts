// Settings · Metrics — what each metric's computation state IS, and the one
// control that changes it.
//
// ## The registry answers "why", not this module
//
// Every "why is there no row for today?" line comes from `sensei.reason_codes`,
// served with the read. Nothing here hardcodes a summary, a remedy, or an
// ordering: `precedence` decides what to say first, and `kind` decides whether it
// reads as fine (`normal`), somebody's decision (`refusal`), or a problem
// (`fault`). That is the whole point of the registry being one table — a second
// vocabulary in the client would drift from it immediately.
//
// So an UNRESOLVED code is reported as unresolved. The tempting alternative — a
// generic "not computed yet" fallback — reads as a real explanation and hides the
// fact that the client and the daemon disagree about the vocabulary.
//
// ## The toggle is a proxy for the tenant's decision
//
// `dojo.metric_activations` is the tenant's record. The daemon only mirrors the
// consequence back through the sync plan, so:
//   - the response is the dōjō's RE-READ ruling, adopted verbatim, never the
//     click's assumption;
//   - a refused write leaves the control alone, so it keeps showing the stored
//     decision rather than what someone wanted;
//   - the row's own `deactivated` (the local mirror) lags until the next sync,
//     which the screen says rather than papering over.

import type {
  MetricReason,
  MetricStatusResponse,
  MetricStatusRow,
  MetricStatusSummary,
  MetricStatusSummaryRow,
} from '$lib/types.js';

/** The API surface this controller needs — the seam, so it unit-tests without a
 *  daemon and never reaches for a global. */
export interface MetricStatusApi {
  tryGetMetricStatus: (
    repo: string,
  ) => Promise<
    { ok: true; data: MetricStatusResponse } | { ok: false; error: { message: string } }
  >;
  patchMetricActivation: (
    persona: string,
    repoKey: string,
    metric: string,
    enabled: boolean,
  ) => Promise<
    | { ok: true; data: { metric: string; enabled: boolean } }
    | { ok: false; error: { message: string } }
  >;
}

/**
 * How much a reason kind demands attention. Lower sorts first.
 *
 * `kind` is the registry's own axis for telling fine from broken, and it has to
 * lead the ranking because `precedence` in THIS domain does not encode severity —
 * measured against the live seed:
 *
 * ```
 * not_yet_effective normal  10      never_computed normal 30    sealed normal 50
 * retired           normal  11      walked         normal 40    behind fault  60
 * deactivated       refusal 20
 * ```
 *
 * That is the ORDER THE VIEW EVALUATES IN (registry lifecycle, then the tenant's
 * choice, then coverage), which is what "first match wins" needs. It is not "fix
 * this first": `behind` is the only `fault` in the domain and it sorts LAST. So
 * ranking on precedence alone headlines "This metric is not in service yet" on a
 * repository whose metric group is actually failing.
 */
const KIND_RANK: Record<MetricReason['kind'], number> = {
  fault: 0,
  refusal: 1,
  normal: 2,
};

/**
 * The reason that most needs saying.
 *
 * `kind` first (a fault outranks a refusal outranks a self-clearing state), then
 * `precedence` ascending WITHIN a kind — which is where the registry's ordering is
 * meaningful. See [`KIND_RANK`] for why precedence cannot lead.
 *
 * Ranking by COUNT — the tempting shortcut — would surface `sealed` on a
 * repository whose metrics were all switched off, because 24 fine metrics
 * outnumber 2 deactivated ones.
 *
 * A code the registry does not know is SKIPPED rather than ranked. Treating a
 * missing precedence as 0 would let an unknown code outrank every real reason.
 * `null` for an empty tally — never a fabricated all-clear.
 */
export function worstReason(
  byReason: Record<string, number>,
  reasons: Record<string, MetricReason>,
): MetricReason | null {
  let worst: MetricReason | null = null;
  for (const code of Object.keys(byReason)) {
    const r = reasons[code];
    if (!r) continue;
    if (!worst) {
      worst = r;
      continue;
    }
    const rank = KIND_RANK[r.kind] ?? Number.MAX_SAFE_INTEGER;
    const best = KIND_RANK[worst.kind] ?? Number.MAX_SAFE_INTEGER;
    if (rank < best || (rank === best && r.precedence < worst.precedence)) worst = r;
  }
  return worst;
}

/** A reason resolved for display. Same shape whether or not the registry knew
 *  the code, so callers never branch on "did it resolve". */
export interface ReasonLine {
  code: string;
  kind: MetricReason['kind'];
  summary: string;
  detail: string;
  remedy: string | null;
  /** True when the registry had no entry — the row says so instead of pretending. */
  unresolved: boolean;
}

/**
 * Resolve a `reason_code` through the served registry.
 *
 * An unknown code becomes a `fault` that NAMES itself, with no remedy. Two
 * things this deliberately does not do: render the bare slug as if it were a
 * sentence, and substitute a plausible line like "not computed yet" — which
 * would read as an explanation while actually meaning "the client is out of step
 * with the daemon's vocabulary".
 */
export function reasonLine(
  code: string,
  reasons: Record<string, MetricReason>,
): ReasonLine {
  const r = reasons[code];
  if (r) {
    return {
      code,
      kind: r.kind,
      summary: r.summary,
      detail: r.detail,
      remedy: r.remedy,
      unresolved: false,
    };
  }
  return {
    code,
    kind: 'fault',
    summary: `Unrecognised reason code "${code}"`,
    detail:
      'The daemon reported a reason this build of the app does not know. ' +
      'The metric state is real; only the explanation is missing.',
    remedy: null,
    unresolved: true,
  };
}

/** How a group settles. Read off the DATA by the view (`last_sha`), so this
 *  follows it — the commit cadence is documented but unimplemented today. */
export function cadenceLabel(cadence: MetricStatusRow['cadence']): string {
  return cadence === 'commit' ? 'per commit' : 'daily';
}

/**
 * How far coverage has got, in words.
 *
 * A day-cadence group settles CALENDAR DAYS: today is never sealed (it reopens as
 * late sessions land), and an empty day still seals — so a gap in the values is
 * not a gap in coverage, and this says "settled through" rather than "last
 * value". No watermark at all is "never run", not a blank cell.
 */
export function watermarkLabel(row: MetricStatusRow): string {
  if (row.cadence === 'commit' && row.last_sha) {
    return `walked to ${row.last_sha.slice(0, 7)}`;
  }
  if (row.sealed_through) return `settled through ${row.sealed_through}`;
  return 'never run';
}

/**
 * Whether a repository's metric activation can be changed at all.
 *
 * Activation is decided per `repo_key` by the dōjō that shares the repository. A
 * local-only repository has no remote, so no key, so no dōjō — its metrics
 * compute normally but there is nobody to rule on them. The screen states that;
 * it does not render a dead toggle.
 */
export function isConfigurable(repoKey: string | null): boolean {
  return repoKey !== null && repoKey !== '';
}

/**
 * The Keychain session slot to sign the activation write with, or null.
 *
 * `PersonaWire.sessionSlot` is the slot; `label` is a display string and
 * explicitly NOT it. Signing against the label addresses a different credential
 * or none, so the daemon answers 401 and the screen would report a permissions
 * problem for what is really a wrong-field bug.
 *
 * `sessionSlot` is null until a persona has connected, so null here means nobody
 * is signed in — a real state the caller must show, not a reason to fall back to
 * `'default'` and let the write fail at the daemon.
 *
 * Takes the first persona that HAS a slot. An install with several signed-in
 * personas will need a picker; until then the first is the honest choice and the
 * screen names which one it used.
 */
export function signedInSlot(
  personas: readonly { sessionSlot: string | null }[],
): string | null {
  return personas.find((p) => p.sessionSlot !== null)?.sessionSlot ?? null;
}

/** One repository in the rail, with the line that says how it is doing. */
export interface RepoEntry extends MetricStatusSummaryRow {
  worst: MetricReason | null;
  configurable: boolean;
}

/** Pure: the summary as rail entries, worst-first line attached. Order is the
 *  daemon's (repository name), which is already deterministic. */
export function repoEntries(summary: MetricStatusSummary): RepoEntry[] {
  return summary.repositories.map((r) => ({
    ...r,
    worst: worstReason(r.by_reason, summary.reasons),
    configurable: isConfigurable(r.repo_key),
  }));
}

/** One metric row prepared for display. */
export interface MetricRowView extends MetricStatusRow {
  reason: ReasonLine;
  cadenceText: string;
  watermarkText: string;
  /** The tenant's ruling: absence of a deactivation IS enabled. */
  enabled: boolean;
}

/**
 * The Settings · Metrics controller.
 *
 * Holds the summary (loaded by `+page.ts`) plus the currently-selected
 * repository's rows. The api client and the persona slot are injected, so this
 * tests without a daemon and without a Keychain.
 */
export class MetricSettings {
  /** Rail entries — every repository, worst reason attached. */
  entries = $state<RepoEntry[]>([]);
  /** The selected repository's uuid, or null before a selection. */
  selected = $state<string | null>(null);
  /** The selected repository's metric rows, ready to render. */
  rows = $state<MetricRowView[]>([]);
  /** True while the per-repository read is in flight. */
  loading = $state(false);
  /** True while an activation write is in flight — one at a time. */
  saving = $state(false);
  /** The last read or write error, or null. Cleared at the start of each. */
  error = $state<string | null>(null);
  /**
   * Bumped whenever a write is REFUSED. The template keys each switch on it.
   *
   * The switch is controlled from `rows`. A refused write leaves `enabled`
   * untouched — correct, since the tenant's ruling did not change — but that
   * means the prop does not change either, so nothing re-renders and the switch
   * keeps showing the position that was clicked. A toggle that visually sticks
   * while nothing was stored is the worst outcome available: it reads as success.
   * Bumping this remounts the control at the authoritative value.
   *
   * Not bumped on success, where `enabled` changes and the re-render is free.
   */
  revision = $state(0);

  #reasons: Record<string, MetricReason>;
  #api: MetricStatusApi;
  // Both are `$state` because the getters below derive from them: `canConfigure`
  // and `blockedReason` must re-evaluate when the persona registry finishes
  // loading (setPersona) and when a repository is selected (select), or the
  // switches stay disabled after the reason for disabling them is gone.
  #repoKey = $state<string | null>(null);
  #persona = $state<string | null>(null);

  /** `persona` is the Keychain session slot, or null when nobody is signed in —
   *  see [`signedInSlot`]. Null is carried through rather than defaulted, so the
   *  screen can say "sign in" instead of showing a 401 as a permissions refusal. */
  constructor(
    summary: MetricStatusSummary,
    api: MetricStatusApi,
    persona: string | null,
  ) {
    this.entries = repoEntries(summary);
    this.#reasons = summary.reasons;
    this.#api = api;
    this.#persona = persona;
  }

  /**
   * Adopt the signed-in slot once the persona registry has loaded.
   *
   * The controller is constructed with `null` because the summary arrives from
   * `load()` while the persona list is fetched client-side — so for the first
   * frames nobody is signed in as far as this screen knows, which is the honest
   * state to render rather than an optimistic guess.
   */
  setPersona(slot: string | null): void {
    this.#persona = slot;
  }

  /** The selected repository's rail entry, or null. */
  get current(): RepoEntry | null {
    return this.entries.find((e) => e.repository_id === this.selected) ?? null;
  }

  /**
   * Whether the selected repository's metrics can be switched at all.
   *
   * Two independent reasons it may not be: the repository has no remote (so no
   * dōjō owns a ruling on it), or nobody is signed in (so there is no credential
   * to write with). The screen distinguishes them — see [`blockedReason`] — but
   * either one disables the control.
   */
  get canConfigure(): boolean {
    return isConfigurable(this.#repoKey) && this.#persona !== null;
  }

  /**
   * Why the controls are unavailable, or null when they are available.
   *
   * Separate from `canConfigure` because the two causes need different sentences:
   * "sign in" is actionable by the reader; "this repository has no remote" is a
   * fact about the repository. Collapsing them into one greyed-out switch is what
   * makes a settings screen feel broken.
   */
  get blockedReason(): string | null {
    if (this.#persona === null) {
      return 'No persona is signed in, so there is no credential to record a change with. Sign in under Settings · Assistants first.';
    }
    if (!isConfigurable(this.#repoKey)) {
      return 'This repository has no remote, so no dōjō can rule on its metrics. They compute locally and cannot be switched off.';
    }
    return null;
  }

  /** Whether one metric is currently on for the selected repository. */
  isEnabled(metric: string): boolean {
    return this.rows.find((r) => r.metric === metric)?.enabled ?? true;
  }

  /**
   * Load one repository's metric rows.
   *
   * Addressed by UUID, not `repo_key`: a local-only repository has no key, and
   * `?repo=` would then be empty and resolve to nothing (or, worse, to another
   * repository). A failed read sets `error` and leaves `rows` EMPTY — the screen
   * shows the error, because an empty metric list would read as "nothing to
   * configure here".
   */
  async select(repositoryId: string): Promise<void> {
    this.selected = repositoryId;
    this.loading = true;
    this.error = null;
    this.rows = [];
    const res = await this.#api.tryGetMetricStatus(repositoryId);
    this.loading = false;
    if (!res.ok) {
      this.error = res.error.message;
      this.#repoKey = null;
      return;
    }
    // The reason vocabulary travels with every read; adopt the fresh one rather
    // than keeping the summary's copy, so a daemon that gained a code mid-session
    // resolves it.
    this.#reasons = res.data.reasons;
    this.#repoKey = res.data.repo_key;
    this.rows = res.data.metrics.map((row) => this.#view(row));
  }

  /**
   * Switch one metric on or off for the selected repository.
   *
   * Refuses without a remote key rather than sending a request that cannot mean
   * anything. Adopts the dōjō's re-read ruling; on a refusal the row is left
   * exactly as it was, so the control keeps showing the stored decision.
   */
  async toggle(metric: string): Promise<boolean> {
    if (this.saving) return false; // one write at a time
    // Refuse locally rather than sending a request that cannot mean anything: a
    // missing slot earns a 401 that reads as "not allowed", and a missing repo_key
    // names no repository the dōjō can rule on.
    const blocked = this.blockedReason;
    if (blocked || !this.#repoKey || !this.#persona) {
      this.error = blocked;
      return false;
    }
    const next = !this.isEnabled(metric);
    this.saving = true;
    this.error = null;
    const res = await this.#api.patchMetricActivation(
      this.#persona,
      this.#repoKey,
      metric,
      next,
    );
    this.saving = false;
    if (!res.ok) {
      this.error = res.error.message; // rows untouched — the ruling did not change
      this.revision += 1; // …so force the control back to the stored value
      return false;
    }
    // The dōjō RE-READS rather than echoing, so adopt what it reports — not
    // `next`. `deactivated` (the local mirror) is deliberately left alone: it
    // only changes on the next sync, and the screen says so.
    const ruled = res.data;
    this.rows = this.rows.map((r) =>
      r.metric === ruled.metric ? { ...r, enabled: ruled.enabled } : r,
    );
    return true;
  }

  #view(row: MetricStatusRow): MetricRowView {
    return {
      ...row,
      reason: reasonLine(row.reason_code, this.#reasons),
      cadenceText: cadenceLabel(row.cadence),
      watermarkText: watermarkLabel(row),
      enabled: !row.deactivated,
    };
  }
}
