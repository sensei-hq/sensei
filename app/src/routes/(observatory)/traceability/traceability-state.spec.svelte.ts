import { describe, it, expect } from 'vitest';
import type { DriftItem } from '$lib/types.js';
import {
  driftTone,
  driftChipClasses,
  rollupDrift,
  groupDriftByProject,
  rollupHeadline,
  toDriftRow,
  type DriftRow,
} from './traceability-state.svelte.js';

const row = (over: Partial<DriftRow> = {}): DriftRow => ({
  id: 'd',
  projectId: 'p',
  projectName: 'sensei',
  detail: 'Mentions `foo` which is not in the code.',
  status: 'broken',
  ...over,
});

describe('toDriftRow', () => {
  it('annotates a wire DriftItem with its owning project', () => {
    const item = {
      id: 'ac55',
      file: '',
      status: 'broken',
      confidence: 0,
      detail: 'Mentions `bar`.',
      expectedSignature: null,
      actualSignature: null,
    } as DriftItem;
    const r = toDriftRow(item, { id: 'proj-1', name: 'rokkit' });
    expect(r).toEqual({
      id: 'ac55',
      projectId: 'proj-1',
      projectName: 'rokkit',
      detail: 'Mentions `bar`.',
      status: 'broken',
    });
  });

  it('falls back to an empty detail when the wire omits it', () => {
    const item = { id: 'x', status: 'broken' } as unknown as DriftItem;
    expect(toDriftRow(item, { id: 'p', name: 'n' }).detail).toBe('');
  });
});

describe('driftTone', () => {
  it('maps broken → danger', () => expect(driftTone('broken')).toBe('danger'));
  it('maps drifted → warning', () => expect(driftTone('drifted')).toBe('warning'));
  it('degrades an unexpected status to ink (never dropped)', () =>
    expect(driftTone('flapping')).toBe('ink'));
});

describe('driftChipClasses', () => {
  it('broken → danger tokens', () =>
    expect(driftChipClasses('broken')).toEqual({ bg: 'bg-danger-soft', text: 'text-danger' }));
  it('drifted → warning tokens', () =>
    expect(driftChipClasses('drifted')).toEqual({ bg: 'bg-warning-soft', text: 'text-warning' }));
  it('unknown → muted ink tokens', () =>
    expect(driftChipClasses('whatever')).toEqual({ bg: 'bg-paper-mute', text: 'text-ink-mute' }));
});

describe('rollupDrift', () => {
  it('counts total, broken, drifted and distinct affected projects', () => {
    const r = rollupDrift([
      row({ id: '1', projectId: 'a', status: 'broken' }),
      row({ id: '2', projectId: 'a', status: 'drifted' }),
      row({ id: '3', projectId: 'b', status: 'broken' }),
    ]);
    expect(r).toEqual({ total: 3, broken: 2, drifted: 1, projectsAffected: 2 });
  });

  it('is all-zero for an empty feed', () => {
    expect(rollupDrift([])).toEqual({ total: 0, broken: 0, drifted: 0, projectsAffected: 0 });
  });
});

describe('groupDriftByProject', () => {
  it('groups rows by project and counts per-status', () => {
    const groups = groupDriftByProject([
      row({ id: '1', projectId: 'a', projectName: 'alpha', status: 'broken' }),
      row({ id: '2', projectId: 'a', projectName: 'alpha', status: 'drifted' }),
      row({ id: '3', projectId: 'b', projectName: 'bravo', status: 'broken' }),
    ]);
    const alpha = groups.find((g) => g.projectId === 'a');
    expect(alpha).toMatchObject({ projectName: 'alpha', broken: 1, drifted: 1 });
    expect(alpha?.rows).toHaveLength(2);
  });

  it('orders groups broken-first (most broken refs lead)', () => {
    const groups = groupDriftByProject([
      row({ id: '1', projectId: 'a', projectName: 'alpha', status: 'broken' }),
      row({ id: '2', projectId: 'b', projectName: 'bravo', status: 'broken' }),
      row({ id: '3', projectId: 'b', projectName: 'bravo', status: 'broken' }),
    ]);
    expect(groups.map((g) => g.projectId)).toEqual(['b', 'a']);
  });

  it('breaks a broken-count tie by total rows, then project name', () => {
    const groups = groupDriftByProject([
      // zulu: 1 broken + 1 drifted = 2 rows
      row({ id: '1', projectId: 'z', projectName: 'zulu', status: 'broken' }),
      row({ id: '2', projectId: 'z', projectName: 'zulu', status: 'drifted' }),
      // mike: 1 broken = 1 row
      row({ id: '3', projectId: 'm', projectName: 'mike', status: 'broken' }),
      // alpha: 1 broken = 1 row (ties mike on broken+total → name breaks it)
      row({ id: '4', projectId: 'a', projectName: 'alpha', status: 'broken' }),
    ]);
    expect(groups.map((g) => g.projectId)).toEqual(['z', 'a', 'm']);
  });

  it('within a group, orders broken rows before drifted', () => {
    const groups = groupDriftByProject([
      row({ id: '1', projectId: 'a', status: 'drifted', detail: 'b' }),
      row({ id: '2', projectId: 'a', status: 'broken', detail: 'a' }),
    ]);
    expect(groups[0].rows.map((r) => r.status)).toEqual(['broken', 'drifted']);
  });
});

describe('rollupHeadline', () => {
  it('renders broken-only with plural agreement', () => {
    expect(rollupHeadline({ total: 153, broken: 153, drifted: 0, projectsAffected: 4 })).toBe(
      '153 broken references across 4 projects.',
    );
  });

  it('uses singular for a single reference and a single project', () => {
    expect(rollupHeadline({ total: 1, broken: 1, drifted: 0, projectsAffected: 1 })).toBe(
      '1 broken reference across 1 project.',
    );
  });

  it('joins broken and drifted when both are present', () => {
    expect(rollupHeadline({ total: 7, broken: 5, drifted: 2, projectsAffected: 3 })).toBe(
      '5 broken references and 2 drifted references across 3 projects.',
    );
  });

  it('falls back to a plain reference count when neither status is set', () => {
    expect(rollupHeadline({ total: 4, broken: 0, drifted: 0, projectsAffected: 2 })).toBe(
      '4 references across 2 projects.',
    );
  });
});
