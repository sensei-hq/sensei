import { describe, it, expect, vi } from 'vitest';
import type { SenseiApi } from '$lib/api.js';
import type { DojoBindingSuggestion, DojoMembership } from '$lib/types.js';
import {
  BindingAction,
  bindingKanji,
  boundMembership,
  confirmedChip,
  inferredChip,
  resolveBinding,
  type BindableWorkspace,
  type InferredBinding,
} from './about-binding-state.svelte.js';

const MEMBERSHIP_ID = '11111111-2222-4333-8444-555555555555';

const membership = (over: Partial<DojoMembership> = {}): DojoMembership => ({
  id: MEMBERSHIP_ID,
  registry_url: 'https://dojo.acme.internal',
  tenant_key: 'github/acme',
  dojo_url: 'https://dojo.acme.internal/github/acme',
  kind: 'client',
  role: 'contributor',
  authenticated_via: 'device_code',
  attribution_default: 'named',
  org_slugs: ['acme'],
  sync_status: 'healthy',
  last_seq: 0,
  last_heartbeat_at: null,
  enabled: true,
  bound_workspaces: [],
  ...over,
});

const suggestion = (over: Partial<DojoBindingSuggestion> = {}): DojoBindingSuggestion => ({
  membership_id: MEMBERSHIP_ID,
  kind: 'client',
  matched_slug: 'acme',
  tenant_key: 'github/acme',
  dojo_url: 'https://dojo.acme.internal/github/acme',
  ...over,
});

const workspace = (over: Partial<BindableWorkspace> = {}): BindableWorkspace => ({
  id: 'proj-1',
  ...over,
});

describe('bindingKanji', () => {
  it('maps each kind to its glyph (reused from the connections map)', () => {
    expect(bindingKanji('client')).toBe('客');
    expect(bindingKanji('employer')).toBe('社');
    expect(bindingKanji('community')).toBe('群');
    expect(bindingKanji('personal')).toBe('己');
  });
  it('falls back for an unknown kind rather than dropping it', () => {
    expect(bindingKanji('federation')).toBe('結');
  });
});

describe('confirmedChip / inferredChip', () => {
  it('confirmed reads the success tone with a sentence-case label', () =>
    expect(confirmedChip()).toEqual({
      bg: 'bg-success-soft',
      text: 'text-success',
      label: 'confirmed',
    }));
  it('inferred reads the warning tone with a sentence-case label', () =>
    expect(inferredChip()).toEqual({
      bg: 'bg-warning-soft',
      text: 'text-warning',
      label: 'inferred',
    }));
});

describe('boundMembership', () => {
  it('resolves via workspaces.dojo_id when the list endpoint carries it', () => {
    const m = membership();
    expect(boundMembership(workspace({ dojo_id: MEMBERSHIP_ID }), [m])).toBe(m);
  });
  it('resolves via a membership bound_workspaces strip (the reliable wire path)', () => {
    const m = membership({ bound_workspaces: [{ id: 'proj-1', name: 'lumen-auth' }] });
    expect(boundMembership(workspace(), [m])).toBe(m);
  });
  it('returns null when unbound by either signal', () => {
    expect(boundMembership(workspace(), [membership()])).toBeNull();
  });
  it('does not match a dojo_id with no corresponding membership', () => {
    expect(boundMembership(workspace({ dojo_id: 'ghost' }), [membership()])).toBeNull();
  });
});

describe('resolveBinding', () => {
  // #262: a failed read is an error, never "no binding" — the empty row would
  // tell you the workspace stays local when the daemon simply did not answer.
  it('error: a failed read wins over every other state', () => {
    const view = resolveBinding(workspace({ dojo_id: MEMBERSHIP_ID }), null, [membership()], 'daemon unreachable');
    expect(view).toEqual({ state: 'error', message: 'daemon unreachable' });
  });

  it('confirmed: a bound workspace reads the membership tenant key + url', () => {
    const view = resolveBinding(
      workspace({ dojo_id: MEMBERSHIP_ID }),
      null,
      [membership()],
    );
    expect(view).toEqual({
      state: 'confirmed',
      kanji: '客',
      tenantKey: 'github/acme',
      dojoUrl: 'https://dojo.acme.internal/github/acme',
    });
  });

  it('confirmed wins over an inferred suggestion (a bound workspace never re-prompts)', () => {
    const view = resolveBinding(
      workspace(),
      suggestion(),
      [membership({ bound_workspaces: [{ id: 'proj-1', name: 'lumen-auth' }] })],
    );
    expect(view.state).toBe('confirmed');
  });

  it('inferred: an unbound workspace with a suggestion carries the matched slug', () => {
    const view = resolveBinding(workspace(), suggestion(), [membership()]);
    expect(view).toEqual({
      state: 'inferred',
      kanji: '客',
      membershipId: MEMBERSHIP_ID,
      matchedSlug: 'acme',
      tenantKey: 'github/acme',
      dojoUrl: 'https://dojo.acme.internal/github/acme',
    });
  });

  it('empty: no binding and no suggestion', () => {
    expect(resolveBinding(workspace(), null, [])).toEqual({ state: 'empty' });
  });
});

// Hand-rolled mock — only `bindWorkspaceDojo` is exercised by the action.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    bindWorkspaceDojo: vi.fn().mockResolvedValue({ ok: true, data: { ok: true, dojo_id: MEMBERSHIP_ID } }),
    ...overrides,
  } as unknown as SenseiApi;
}

const inferred = (): InferredBinding => ({
  state: 'inferred',
  kanji: '客',
  membershipId: MEMBERSHIP_ID,
  matchedSlug: 'acme',
  tenantKey: 'github/acme',
  dojoUrl: 'https://dojo.acme.internal/github/acme',
});

describe('BindingAction', () => {
  it('confirm() posts the membership id and swaps in the confirmed view on ok', async () => {
    const bindWorkspaceDojo = vi
      .fn()
      .mockResolvedValue({ ok: true, data: { ok: true, dojo_id: MEMBERSHIP_ID } });
    const action = new BindingAction();

    const ok = await action.confirm(mockApi({ bindWorkspaceDojo }), 'proj-1', inferred());
    expect(ok).toBe(true);
    expect(bindWorkspaceDojo).toHaveBeenCalledWith('proj-1', MEMBERSHIP_ID);
    expect(action.error).toBeNull();
    expect(action.pending).toBe(false);
    expect(action.confirmed).toEqual({
      state: 'confirmed',
      kanji: '客',
      tenantKey: 'github/acme',
      dojoUrl: 'https://dojo.acme.internal/github/acme',
    });
  });

  it('confirm() surfaces the wire error and leaves confirmed null on failure', async () => {
    const bindWorkspaceDojo = vi
      .fn()
      .mockResolvedValue({ ok: false, error: { status: 404, message: 'unknown membership' } });
    const action = new BindingAction();

    const ok = await action.confirm(mockApi({ bindWorkspaceDojo }), 'proj-1', inferred());
    expect(ok).toBe(false);
    expect(action.error).toBe('unknown membership');
    expect(action.confirmed).toBeNull();
    expect(action.pending).toBe(false);
  });
});
