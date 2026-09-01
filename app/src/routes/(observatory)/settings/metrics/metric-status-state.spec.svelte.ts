// Settings · Metrics — the state module's pure logic and the toggle controller.
//
// The load-bearing properties here are all about NOT lying to the reader:
//   - a reason code is resolved through the served registry, never rendered raw
//     and never given an invented fallback line;
//   - ranking uses the registry's `precedence`, not a hardcoded order;
//   - a repository with no remote key cannot be toggled, and the reason is shown
//     rather than the control being silently disabled;
//   - a failed PATCH leaves the toggle where it was, so the control reflects the
//     tenant's actual ruling and not the click.
import { describe, it, expect, vi } from 'vitest';
import {
  worstReason,
  reasonLine,
  cadenceLabel,
  watermarkLabel,
  isConfigurable,
  signedInSlot,
  MetricSettings,
} from './metric-status-state.svelte.js';
import type {
  MetricReason,
  MetricStatusResponse,
  MetricStatusSummary,
} from '$lib/types.js';

// The REAL seeded vocabulary — kinds, precedences and summaries copied from
// `sensei.reason_codes` where domain = 'metric_computation', verified against the
// live database. An earlier version of this file invented precedences (2/4/5/6)
// and called `never_computed` a fault; every ranking test then passed against
// data that does not exist, and hid the fact that `behind` — the domain's ONLY
// fault — carries the HIGHEST precedence and so sorts last.
//
//   not_yet_effective normal  10     never_computed normal 30    sealed normal 50
//   retired           normal  11     walked         normal 40    behind fault  60
//   deactivated       refusal 20
const REASONS: Record<string, MetricReason> = {
  not_yet_effective: {
    code: 'not_yet_effective',
    kind: 'normal',
    precedence: 10,
    summary: 'This metric is not in service yet',
    detail: 'detail',
    remedy: null,
    actor: null,
  },
  retired: {
    code: 'retired',
    kind: 'normal',
    precedence: 11,
    summary: 'This metric has been retired',
    detail: 'detail',
    remedy: null,
    actor: null,
  },
  deactivated: {
    code: 'deactivated',
    kind: 'refusal',
    precedence: 20,
    summary: 'Every dojo sharing this repository switched this metric off',
    detail: 'detail',
    remedy: 'Turn it back on for at least one dojo',
    actor: 'user',
  },
  never_computed: {
    code: 'never_computed',
    kind: 'normal',
    precedence: 30,
    summary: 'This metric has never run for this repository',
    detail: 'detail',
    remedy: null,
    actor: null,
  },
  sealed: {
    code: 'sealed',
    kind: 'normal',
    precedence: 50,
    summary: 'Calendar days are settled through yesterday',
    detail: 'detail',
    remedy: null,
    actor: null,
  },
  behind: {
    code: 'behind',
    kind: 'fault',
    precedence: 60,
    summary: 'Calendar days are unsettled further back than yesterday',
    detail: 'detail',
    remedy: 'Check the metrics task for a failing group',
    actor: 'user',
  },
};

function row(over: Partial<MetricStatusResponse['metrics'][0]> = {}) {
  return {
    repository_id: 'r-1',
    repo_key: 'github.com/acme/api',
    repository_name: 'Api',
    metric: 'ftr',
    metric_group: 'session_outcomes',
    cadence: 'day' as const,
    sealed_through: '2026-08-30',
    last_sha: null,
    watermark_updated_at: '2026-08-31T04:00:00Z',
    effective_from: '2026-01-01',
    effective_until: null,
    deactivated: false,
    deactivated_observed_at: null,
    reason_code: 'sealed',
    last_computed_on: '2026-08-30',
    ...over,
  };
}

describe('worstReason', () => {
  it('ranks a fault above everything, even though its precedence is the HIGHEST', () => {
    // The load-bearing case, and the one that fabricated fixtures hid. In the real
    // seed `behind` is the only fault and carries precedence 60 — the largest in
    // the domain — because precedence encodes the VIEW's evaluation order
    // (lifecycle → choice → coverage), not severity. Ranking on precedence alone
    // headlines "This metric is not in service yet" for a repository whose metric
    // group is failing.
    const worst = worstReason(
      { not_yet_effective: 1, sealed: 24, behind: 2 },
      REASONS,
    );
    expect(worst?.code).toBe('behind');
  });

  it('ranks a refusal above a self-clearing state', () => {
    // `deactivated` (refusal, 20) over `never_computed` (normal, 30) and `sealed`
    // (normal, 50): somebody decided this, and that outranks progress.
    const worst = worstReason({ sealed: 24, never_computed: 3, deactivated: 2 }, REASONS);
    expect(worst?.code).toBe('deactivated');
  });

  it('does not rank by count', () => {
    // 24 fine metrics outnumber 2 switched-off ones. Sorting by count would report
    // "everything is fine" on a repository whose metrics were deliberately stopped.
    expect(worstReason({ sealed: 24, deactivated: 2 }, REASONS)?.code).toBe('deactivated');
  });

  it('falls back to precedence WITHIN a kind, where the registry ordering is real', () => {
    // Both `normal`: retired (11) beats never_computed (30) beats sealed (50).
    expect(
      worstReason({ sealed: 24, never_computed: 3, retired: 2 }, REASONS)?.code,
    ).toBe('retired');
  });

  it('ignores a code the registry does not know rather than ranking it first', () => {
    // An unresolvable code has no kind and no precedence. Treating a missing one
    // as 0 would make an unknown code outrank every real reason.
    const worst = worstReason({ mystery: 9, sealed: 1 }, REASONS);
    expect(worst?.code).toBe('sealed');
  });

  it('is null for an empty tally, not a fabricated all-clear', () => {
    expect(worstReason({}, REASONS)).toBeNull();
  });
});

describe('reasonLine', () => {
  it('resolves a code to its registry summary', () => {
    expect(reasonLine('sealed', REASONS).summary).toBe(
      'Calendar days are settled through yesterday',
    );
  });

  it('carries the remedy for a refusal and null for a self-clearing reason', () => {
    expect(reasonLine('deactivated', REASONS).remedy).toBe(
      'Turn it back on for at least one dojo',
    );
    // A `normal` code carries no remedy by DDL invariant. Inventing one ("wait
    // for the next run") would be a fabricated instruction.
    expect(reasonLine('sealed', REASONS).remedy).toBeNull();
  });

  it('surfaces an unresolved code AS unresolved rather than inventing a line', () => {
    const line = reasonLine('not_in_registry', REASONS);
    expect(line.summary).toContain('not_in_registry');
    expect(line.kind).toBe('fault');
    expect(line.remedy).toBeNull();
  });
});

describe('cadenceLabel + watermarkLabel', () => {
  it('reads the day cadence as settled-through, which is what sealed means', () => {
    expect(cadenceLabel('day')).toBe('daily');
    expect(watermarkLabel(row())).toBe('settled through 2026-08-30');
  });

  it('says never rather than showing a blank for a metric with no watermark', () => {
    expect(watermarkLabel(row({ sealed_through: null, reason_code: 'never_computed' }))).toBe(
      'never run',
    );
  });

  it('names the snapshot cadence, and dates it by the last value', () => {
    // A snapshot group (cost / coverage / knowledge) computes current state only
    // and keeps NO watermark by design, so there is no settled-through day. Saying
    // "never run" of it was the #128 defect; the honest line is when it last
    // produced a value, which the view now carries as `last_computed_on`.
    expect(cadenceLabel('snapshot')).toBe('current state');
    expect(
      watermarkLabel(
        row({ cadence: 'snapshot', sealed_through: null, last_computed_on: '2026-09-01' }),
      ),
    ).toBe('last computed 2026-09-01');
  });

  it('still says never run for a snapshot group that has produced nothing', () => {
    // 189 of the 201 are genuinely this. The new cadence must not turn them into
    // a reassuring line.
    expect(
      watermarkLabel(row({ cadence: 'snapshot', sealed_through: null, last_computed_on: null })),
    ).toBe('never run');
  });

  it('reports a commit cursor when the engine writes one', () => {
    // `last_sha` is null in every live row today — the commit cadence is
    // documented and unimplemented. This pins the branch so the label follows
    // the DATA if that ever changes, rather than needing a code change.
    expect(cadenceLabel('commit')).toBe('per commit');
    expect(
      watermarkLabel(row({ cadence: 'commit', last_sha: 'abc1234def', sealed_through: null })),
    ).toBe('walked to abc1234');
  });
});

describe('isConfigurable', () => {
  it('is false without a remote key, because no dojo can rule on it', () => {
    expect(isConfigurable(null)).toBe(false);
  });
  it('is true with a remote key', () => {
    expect(isConfigurable('github.com/acme/api')).toBe(true);
  });
});

describe('signedInSlot', () => {
  it('takes the sessionSlot, never the label', () => {
    // `PersonaList` exposes both, and `label` is explicitly NOT the slot. Signing
    // against the label addresses a different credential — the daemon then answers
    // 401 and the screen blames permissions for a wrong-field bug.
    expect(
      signedInSlot([{ label: 'Work', sessionSlot: 'slot-7' } as never]),
    ).toBe('slot-7');
  });

  it('skips a persona that has never connected', () => {
    expect(
      signedInSlot([
        { label: 'Never used', sessionSlot: null } as never,
        { label: 'Work', sessionSlot: 'slot-2' } as never,
      ]),
    ).toBe('slot-2');
  });

  it('is null when nobody is signed in, rather than guessing "default"', () => {
    // Falling back to a conventional slot name would send a write that cannot
    // succeed, and the 401 would read as "you are not allowed" instead of
    // "you are not signed in".
    expect(signedInSlot([{ label: 'a', sessionSlot: null } as never])).toBeNull();
    expect(signedInSlot([])).toBeNull();
  });
});

function summary(): MetricStatusSummary {
  return {
    count: 2,
    reasons: REASONS,
    repositories: [
      {
        repository_id: 'r-1',
        repo_key: 'github.com/acme/api',
        name: 'Api',
        by_reason: { sealed: 24, never_computed: 5 },
        total: 29,
      },
      {
        repository_id: 'r-2',
        repo_key: null,
        name: 'Local notes',
        by_reason: { never_computed: 29 },
        total: 29,
      },
    ],
  };
}

function detail(over: Partial<MetricStatusResponse> = {}): MetricStatusResponse {
  return {
    repository_id: 'r-1',
    repo_key: 'github.com/acme/api',
    name: 'Api',
    metrics: [row(), row({ metric: 'rework_ratio', reason_code: 'never_computed' })],
    reasons: REASONS,
    count: 2,
    ...over,
  };
}

describe('MetricSettings', () => {
  const api = (over: Partial<Record<string, unknown>> = {}) => ({
    tryGetMetricStatus: vi.fn().mockResolvedValue({ ok: true, data: detail() }),
    patchMetricActivation: vi
      .fn()
      .mockResolvedValue({
        ok: true,
        data: { repoKey: 'github.com/acme/api', metric: 'ftr', enabled: false, tenant: 'acme' },
      }),
    ...over,
  });

  it('selects a repository and loads its metrics', async () => {
    const a = api();
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-1');
    expect(a.tryGetMetricStatus).toHaveBeenCalledWith('r-1');
    expect(s.rows.map((r) => r.metric)).toEqual(['ftr', 'rework_ratio']);
  });

  it('addresses the repository by UUID, so a keyless one is still openable', async () => {
    const a = api({
      tryGetMetricStatus: vi.fn().mockResolvedValue({
        ok: true,
        data: detail({ repository_id: 'r-2', repo_key: null, name: 'Local notes', metrics: [] }),
      }),
    });
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-2');
    // The uuid, never the null key — `?repo=null` would 404 or, worse, resolve
    // to some other repository.
    expect(a.tryGetMetricStatus).toHaveBeenCalledWith('r-2');
    expect(s.canConfigure).toBe(false);
  });

  it('surfaces a failed load as an error instead of an empty metric list', async () => {
    const a = api({
      tryGetMetricStatus: vi
        .fn()
        .mockResolvedValue({ ok: false, error: { status: 500, message: 'boom' } }),
    });
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-1');
    expect(s.error).toBe('boom');
    expect(s.rows).toEqual([]);
  });

  it('sends the persona SLOT and the repo KEY to the activation write', async () => {
    const a = api();
    const s = new MetricSettings(summary(), a as never, 'slot-7');
    await s.select('r-1');
    await s.toggle('ftr');
    // enabled=false because the row starts enabled (deactivated: false).
    expect(a.patchMetricActivation).toHaveBeenCalledWith(
      'slot-7',
      'github.com/acme/api',
      'ftr',
      false,
    );
  });

  it('adopts the dojo re-read ruling rather than assuming the click won', async () => {
    // The dōjō may answer with something other than what was asked — it re-reads
    // rather than echoing. Trusting the click would drift the control away from
    // the stored decision.
    const a = api({
      patchMetricActivation: vi.fn().mockResolvedValue({
        ok: true,
        data: { repoKey: 'github.com/acme/api', metric: 'ftr', enabled: true, tenant: 'acme' },
      }),
    });
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-1');
    await s.toggle('ftr');
    expect(s.isEnabled('ftr')).toBe(true);
  });

  it('leaves the toggle where it was when the write is refused', async () => {
    const a = api({
      patchMetricActivation: vi.fn().mockResolvedValue({
        ok: false,
        error: { status: 403, message: 'you may not change metric activation' },
      }),
    });
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-1');
    expect(s.isEnabled('ftr')).toBe(true);
    await s.toggle('ftr');
    expect(s.isEnabled('ftr')).toBe(true); // unchanged — reflects the real ruling
    expect(s.error).toBe('you may not change metric activation');
  });

  it('refuses to write with nobody signed in, and says so', async () => {
    // A null slot is a real state, not a reason to guess. Sending anyway earns a
    // 401 that reads as a permissions problem rather than "sign in first".
    const a = api();
    const s = new MetricSettings(summary(), a as never, null);
    await s.select('r-1');
    await s.toggle('ftr');
    expect(a.patchMetricActivation).not.toHaveBeenCalled();
    // The message must tell the reader what to DO, not just that it failed.
    expect(s.error?.toLowerCase()).toContain('sign in');
    expect(s.canConfigure).toBe(false);
  });

  it('refuses to write for a repository with no remote key', async () => {
    const a = api({
      tryGetMetricStatus: vi.fn().mockResolvedValue({
        ok: true,
        data: detail({
          repository_id: 'r-2',
          repo_key: null,
          metrics: [row({ repository_id: 'r-2', repo_key: null })],
        }),
      }),
    });
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-2');
    await s.toggle('ftr');
    expect(a.patchMetricActivation).not.toHaveBeenCalled();
    expect(s.error).toContain('no remote');
  });

  it('bumps a revision on a REFUSED write so the control is remounted', async () => {
    // The switch is a controlled component fed `value={row.enabled}`. When a write
    // is refused, `enabled` does not change — so the prop does not change, so
    // nothing re-renders, and the switch keeps showing the position the user
    // clicked. A toggle that visually stuck while the tenant's ruling never
    // changed is the worst outcome here: it reads as success.
    //
    // The template keys each switch on this revision, so a bump forces a fresh
    // mount at the authoritative value.
    const a = api({
      patchMetricActivation: vi
        .fn()
        .mockResolvedValue({ ok: false, error: { status: 403, message: 'nope' } }),
    });
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-1');
    const before = s.revision;
    await s.toggle('ftr');
    expect(s.revision).toBeGreaterThan(before);
  });

  it('does not bump the revision on success, where the value itself changed', async () => {
    // A successful write changes `enabled`, so the prop change already re-renders.
    // Bumping here too would remount every switch on every click for no reason.
    const a = api();
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-1');
    const before = s.revision;
    await s.toggle('ftr');
    expect(s.revision).toBe(before);
  });

  it('will not start a second write while one is in flight', async () => {
    let release: (v: unknown) => void = () => {};
    const a = api({
      patchMetricActivation: vi.fn().mockReturnValue(new Promise((r) => (release = r))),
    });
    const s = new MetricSettings(summary(), a as never, 'default');
    await s.select('r-1');
    const first = s.toggle('ftr');
    await s.toggle('ftr'); // must be dropped, not queued into a double-flip
    expect(a.patchMetricActivation).toHaveBeenCalledTimes(1);
    release({ ok: true, data: { repoKey: 'k', metric: 'ftr', enabled: false, tenant: 't' } });
    await first;
  });
});
