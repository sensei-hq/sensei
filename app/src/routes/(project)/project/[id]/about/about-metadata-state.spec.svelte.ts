// @vitest-environment jsdom
/**
 * Tests for ProjectMetadataForm — the Project window · About screen's editable
 * metadata form (status/client/goal/description). Uses $state so this is a
 * .spec.svelte.ts. The api client is passed into `save` as the seam, so the
 * whole thing is unit-testable with a hand-rolled mock and never touches a
 * global or the network.
 */
import { describe, it, expect, vi } from 'vitest';
import type { SenseiApi } from '$lib/api.js';
import {
  MATURITY_OPTIONS,
  ProjectMetadataForm,
  saveStatusLabel,
  type EditableProject,
} from './about-metadata-state.svelte.js';

// Hand-rolled mock — only `tryUpdateProject` is exercised by the form.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    tryUpdateProject: vi.fn().mockResolvedValue({ ok: true, data: undefined }),
    ...overrides,
  } as unknown as SenseiApi;
}

const project = (over: Partial<EditableProject> = {}): EditableProject => ({
  id: 'proj-1',
  name: 'lumen-auth',
  client: 'acme',
  goal: 'ship passwordless login',
  description: 'the auth service',
  maturity: 'active',
  ...over,
});

describe('MATURITY_OPTIONS', () => {
  it('exposes exactly the sensei.project_maturity enum values in ladder order', () => {
    expect(MATURITY_OPTIONS.map((o) => o.value)).toEqual([
      'discovery',
      'active',
      'maintenance',
      'archived',
    ]);
  });
  it('gives each option a human sentence-case label', () => {
    expect(MATURITY_OPTIONS.find((o) => o.value === 'discovery')?.label).toBe('Discovery');
    expect(MATURITY_OPTIONS.find((o) => o.value === 'maintenance')?.label).toBe('Maintenance');
  });
});

describe('saveStatusLabel', () => {
  it('maps each status to its copy', () => {
    expect(saveStatusLabel('idle')).toBe('auto-saves as you edit');
    expect(saveStatusLabel('saving')).toBe('saving…');
    expect(saveStatusLabel('saved')).toBe('saved');
  });
  it('surfaces the error message when set, else a generic fallback', () => {
    expect(saveStatusLabel('error', 'boom')).toBe('boom');
    expect(saveStatusLabel('error')).toBe('save failed');
  });
});

describe('ProjectMetadataForm.hydrate', () => {
  it('seeds the fields from the wire project', () => {
    const f = new ProjectMetadataForm();
    f.hydrate(project());
    expect(f.name).toBe('lumen-auth');
    expect(f.client).toBe('acme');
    expect(f.goal).toBe('ship passwordless login');
    expect(f.description).toBe('the auth service');
    expect(f.maturity).toBe('active');
  });

  it('falls back to `vision` for goal when the list payload omits goal', () => {
    // The /api/projects list serializes goal as `vision`; the About load reads
    // from that list, so the form must accept either key.
    const f = new ProjectMetadataForm();
    f.hydrate({ id: 'p', name: 'n', maturity: 'discovery', vision: 'from vision' });
    expect(f.goal).toBe('from vision');
  });

  it('coerces null/undefined wire fields to empty strings', () => {
    const f = new ProjectMetadataForm();
    f.hydrate({ id: 'p', name: 'n', maturity: 'discovery', client: null, goal: null, description: null });
    expect(f.client).toBe('');
    expect(f.goal).toBe('');
    expect(f.description).toBe('');
  });

  it('defaults maturity to discovery when the wire omits it', () => {
    const f = new ProjectMetadataForm();
    f.hydrate({ id: 'p', name: 'n' } as EditableProject);
    expect(f.maturity).toBe('discovery');
  });
});

describe('ProjectMetadataForm.toPatch', () => {
  it('includes every non-empty field so the daemon partial-updates them', () => {
    const f = new ProjectMetadataForm();
    f.hydrate(project());
    expect(f.toPatch()).toEqual({
      name: 'lumen-auth',
      client: 'acme',
      goal: 'ship passwordless login',
      description: 'the auth service',
      maturity: 'active',
    });
  });

  it('omits empty strings so an unset field never clobbers a stored value', () => {
    const f = new ProjectMetadataForm();
    f.hydrate({ id: 'p', name: 'n', maturity: 'active', client: '', goal: '', description: '' });
    expect(f.toPatch()).toEqual({ name: 'n', maturity: 'active' });
  });
});

describe('ProjectMetadataForm.save', () => {
  it('starts idle before any save', () => {
    expect(new ProjectMetadataForm().saveStatus).toBe('idle');
  });

  it('PUTs the current patch and lands on saved on ok', async () => {
    const tryUpdateProject = vi.fn().mockResolvedValue({ ok: true, data: undefined });
    const f = new ProjectMetadataForm();
    f.hydrate(project());
    f.goal = 'new goal';

    const ok = await f.save(mockApi({ tryUpdateProject }), 'proj-1');
    expect(ok).toBe(true);
    expect(tryUpdateProject).toHaveBeenCalledWith(
      'proj-1',
      expect.objectContaining({ goal: 'new goal' }),
    );
    expect(f.saveStatus).toBe('saved');
    expect(f.saveError).toBeNull();
  });

  it('surfaces the wire error and lands on error on failure', async () => {
    const tryUpdateProject = vi
      .fn()
      .mockResolvedValue({ ok: false, error: { status: 400, message: 'bad maturity' } });
    const f = new ProjectMetadataForm();
    f.hydrate(project());

    const ok = await f.save(mockApi({ tryUpdateProject }), 'proj-1');
    expect(ok).toBe(false);
    expect(f.saveStatus).toBe('error');
    expect(f.saveError).toBe('bad maturity');
  });

  it('surfaces a thrown transport error rather than leaving status stuck on saving', async () => {
    const tryUpdateProject = vi.fn().mockRejectedValue(new Error('network down'));
    const f = new ProjectMetadataForm();
    f.hydrate(project());

    const ok = await f.save(mockApi({ tryUpdateProject }), 'proj-1');
    expect(ok).toBe(false);
    expect(f.saveStatus).toBe('error');
    expect(f.saveError).toBe('network down');
  });

  it('no-ops (does not call the api) when there is no project id', async () => {
    const tryUpdateProject = vi.fn();
    const f = new ProjectMetadataForm();
    f.hydrate(project());

    const ok = await f.save(mockApi({ tryUpdateProject }), '');
    expect(ok).toBe(false);
    expect(tryUpdateProject).not.toHaveBeenCalled();
    expect(f.saveStatus).toBe('idle');
  });
});
