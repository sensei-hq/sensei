import { describe, it, expect } from 'vitest';
import type { DojoMembership } from '$lib/types.js';
import {
  boundWorkspacesSummary,
  connectionsHeadline,
  kindPill,
  syncChip,
  syncTone,
} from './dojo-connections-state.svelte.js';

const VALID_UUID = '11111111-2222-4333-8444-555555555555';

const membership = (over: Partial<DojoMembership> = {}): DojoMembership => ({
  id: VALID_UUID,
  registry_url: 'https://dojo.acme.internal',
  tenant_key: 'github/acme',
  dojo_url: 'https://dojo.acme.internal/github/acme',
  kind: 'employer',
  role: 'contributor',
  authenticated_via: 'github_oauth',
  attribution_default: 'named',
  org_slugs: [],
  sync_status: 'healthy',
  last_seq: 0,
  last_heartbeat_at: null,
  enabled: true,
  bound_workspaces: [],
  ...over,
});

describe('syncTone', () => {
  it('maps healthy → success', () => expect(syncTone('healthy')).toBe('success'));
  it('maps stale → warning', () => expect(syncTone('stale')).toBe('warning'));
  it('maps error → danger', () => expect(syncTone('error')).toBe('danger'));
  it('maps authenticating → info', () => expect(syncTone('authenticating')).toBe('info'));
  it('degrades an unexpected status to ink (never dropped)', () =>
    expect(syncTone('flapping')).toBe('ink'));
});

describe('syncChip', () => {
  it('healthy → success tokens, labelled with the raw status', () =>
    expect(syncChip('healthy')).toEqual({
      bg: 'bg-success-soft',
      text: 'text-success',
      label: 'healthy',
    }));
  it('stale → warning tokens', () =>
    expect(syncChip('stale')).toEqual({
      bg: 'bg-warning-soft',
      text: 'text-warning',
      label: 'stale',
    }));
  it('error → danger tokens', () =>
    expect(syncChip('error')).toEqual({
      bg: 'bg-danger-soft',
      text: 'text-danger',
      label: 'error',
    }));
  it('authenticating → info tokens', () =>
    expect(syncChip('authenticating')).toEqual({
      bg: 'bg-info-soft',
      text: 'text-info',
      label: 'authenticating',
    }));
  it('unknown → muted ink tokens, still labelled', () =>
    expect(syncChip('weird')).toEqual({
      bg: 'bg-paper-mute',
      text: 'text-ink-soft',
      label: 'weird',
    }));
});

describe('kindPill', () => {
  it('employer → 社, muted', () =>
    expect(kindPill('employer')).toEqual({
      kanji: '社',
      label: 'employer',
      bg: 'bg-paper-mute',
      text: 'text-ink-soft',
    }));
  it('distinguishes client visually with the accent treatment', () => {
    const pill = kindPill('client');
    expect(pill.kanji).toBe('客');
    expect(pill.bg).toBe('bg-accent-soft');
    expect(pill.text).toBe('text-accent');
  });
  it('community → 群', () => expect(kindPill('community').kanji).toBe('群'));
  it('personal → 己', () => expect(kindPill('personal').kanji).toBe('己'));
  it('falls back for an unknown kind but keeps the raw label', () => {
    const pill = kindPill('federation');
    expect(pill.kanji).toBe('結');
    expect(pill.label).toBe('federation');
    expect(pill.bg).toBe('bg-paper-mute');
  });
});

describe('boundWorkspacesSummary', () => {
  it('none bound', () => expect(boundWorkspacesSummary([])).toBe('no workspaces bound'));
  it('singular', () =>
    expect(boundWorkspacesSummary([{ id: 'a', name: 'lumen-auth' }])).toBe('1 workspace bound'));
  it('plural', () =>
    expect(
      boundWorkspacesSummary([
        { id: 'a', name: 'lumen-auth' },
        { id: 'b', name: 'billing' },
      ]),
    ).toBe('2 workspaces bound'));
});

describe('connectionsHeadline', () => {
  it('counts connections and how many are healthy', () => {
    expect(
      connectionsHeadline([
        membership({ id: '1', sync_status: 'healthy' }),
        membership({ id: '2', sync_status: 'stale' }),
        membership({ id: '3', sync_status: 'healthy' }),
      ]),
    ).toBe('3 connections · 2 healthy.');
  });
  it('uses singular for a single connection', () => {
    expect(connectionsHeadline([membership({ sync_status: 'error' })])).toBe(
      '1 connection · 0 healthy.',
    );
  });
});
