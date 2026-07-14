import { describe, it, expect, vi } from 'vitest';
import type { SenseiApi } from '$lib/api.js';
import type { DojoMembership } from '$lib/types.js';
import {
  ConnectForm,
  boundProjectsSummary,
  connectionsHeadline,
  kindPill,
  parseOrgSlugs,
  syncChip,
  syncTone,
  toConnectBody,
  validateConnect,
  validateRegistryUrl,
  type ConnectDraft,
} from './dojo-connections-state.svelte.js';

const VALID_UUID = '11111111-2222-4333-8444-555555555555';

const membership = (over: Partial<DojoMembership> = {}): DojoMembership => ({
  id: VALID_UUID,
  registry_url: 'https://dojo.acme.internal',
  tenant_key: 'github/acme',
  dojo_url: 'https://dojo.acme.internal/github/acme',
  kind: 'employer',
  role: 'contributor',
  authenticated_via: 'device_code',
  attribution_default: 'named',
  org_slugs: [],
  sync_status: 'healthy',
  last_seq: 0,
  last_heartbeat_at: null,
  enabled: true,
  bound_projects: [],
  ...over,
});

const draft = (over: Partial<ConnectDraft> = {}): ConnectDraft => ({
  membershipId: VALID_UUID,
  tenantKey: 'github/acme',
  registryUrl: '',
  kind: 'employer',
  deviceToken: 'tok_123',
  projectId: '',
  orgSlugs: '',
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
      text: 'text-ink-mute',
      label: 'weird',
    }));
});

describe('kindPill', () => {
  it('employer → 社, muted', () =>
    expect(kindPill('employer')).toEqual({
      kanji: '社',
      label: 'employer',
      bg: 'bg-paper-mute',
      text: 'text-ink-mute',
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

describe('boundProjectsSummary', () => {
  it('none bound', () => expect(boundProjectsSummary([])).toBe('no projects bound'));
  it('singular', () =>
    expect(boundProjectsSummary([{ id: 'a', name: 'lumen-auth' }])).toBe('1 project bound'));
  it('plural', () =>
    expect(
      boundProjectsSummary([
        { id: 'a', name: 'lumen-auth' },
        { id: 'b', name: 'billing' },
      ]),
    ).toBe('2 projects bound'));
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

describe('validateRegistryUrl', () => {
  it('accepts an empty url (the daemon defaults it)', () =>
    expect(validateRegistryUrl('')).toBeNull());
  it('accepts an https url', () =>
    expect(validateRegistryUrl('https://dojo.acme.internal')).toBeNull());
  it('accepts http on loopback', () =>
    expect(validateRegistryUrl('http://localhost:8787')).toBeNull());
  it('rejects a non-loopback http url', () =>
    expect(validateRegistryUrl('http://dojo.acme.internal')).toBe(
      'non-loopback url must be https',
    ));
});

describe('validateConnect', () => {
  it('accepts a complete valid draft', () => expect(validateConnect(draft())).toBeNull());
  it('requires a membership id', () =>
    expect(validateConnect(draft({ membershipId: '  ' }))).toBe('membership id required'));
  it('requires the membership id to be a uuid', () =>
    expect(validateConnect(draft({ membershipId: 'not-a-uuid' }))).toBe(
      'membership id must be the service membership uuid',
    ));
  it('requires a tenant key', () =>
    expect(validateConnect(draft({ tenantKey: '' }))).toBe('tenant key required'));
  it('requires a device token', () =>
    expect(validateConnect(draft({ deviceToken: '' }))).toBe('device token required'));
  it('rejects an insecure registry url', () =>
    expect(validateConnect(draft({ registryUrl: 'http://dojo.acme.internal' }))).toBe(
      'non-loopback url must be https',
    ));
});

describe('parseOrgSlugs', () => {
  it('empty input yields an empty list', () => expect(parseOrgSlugs('')).toEqual([]));
  it('whitespace-only input yields an empty list', () =>
    expect(parseOrgSlugs('   ')).toEqual([]));
  it('splits on commas', () =>
    expect(parseOrgSlugs('sensei-hq,acme')).toEqual(['sensei-hq', 'acme']));
  it('splits on whitespace', () =>
    expect(parseOrgSlugs('sensei-hq acme')).toEqual(['sensei-hq', 'acme']));
  it('splits on mixed commas and whitespace, dropping empties', () =>
    expect(parseOrgSlugs(' sensei-hq ,  acme,, globex ')).toEqual([
      'sensei-hq',
      'acme',
      'globex',
    ]));
  it('lowercases every slug', () =>
    expect(parseOrgSlugs('Sensei-HQ, ACME')).toEqual(['sensei-hq', 'acme']));
  it('dedupes, preserving first-seen order (case-insensitively)', () =>
    expect(parseOrgSlugs('acme, Acme, sensei-hq, acme')).toEqual(['acme', 'sensei-hq']));
});

describe('toConnectBody', () => {
  it('builds a trimmed body and omits blank optionals', () => {
    expect(toConnectBody(draft({ membershipId: ` ${VALID_UUID} `, tenantKey: ' github/acme ' }))).toEqual({
      membership_id: VALID_UUID,
      tenant_key: 'github/acme',
      kind: 'employer',
      credential: 'tok_123',
    });
  });
  it('includes registry_url and project_id when provided', () => {
    expect(
      toConnectBody(
        draft({ registryUrl: 'https://dojo.acme.internal', projectId: 'proj-9', kind: 'client' }),
      ),
    ).toEqual({
      membership_id: VALID_UUID,
      tenant_key: 'github/acme',
      kind: 'client',
      credential: 'tok_123',
      registry_url: 'https://dojo.acme.internal',
      project_id: 'proj-9',
    });
  });
  it('includes normalised org_slugs when provided', () => {
    expect(toConnectBody(draft({ orgSlugs: 'Sensei-HQ, acme, acme' }))).toEqual({
      membership_id: VALID_UUID,
      tenant_key: 'github/acme',
      kind: 'employer',
      credential: 'tok_123',
      org_slugs: ['sensei-hq', 'acme'],
    });
  });
  it('omits org_slugs when the input parses to nothing', () => {
    const body = toConnectBody(draft({ orgSlugs: '  ,  ' }));
    expect(body.org_slugs).toBeUndefined();
  });
});

// Hand-rolled mock api — only `connectDojo` is exercised by the form; cast to
// SenseiApi since the state touches nothing else.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    connectDojo: vi.fn().mockResolvedValue({ ok: true, data: { id: VALID_UUID } }),
    ...overrides,
  } as unknown as SenseiApi;
}

describe('ConnectForm', () => {
  it('validationError reflects the current fields; canSubmit tracks it', () => {
    const form = new ConnectForm();
    expect(form.validationError).toBe('membership id required');
    expect(form.canSubmit).toBe(false);

    form.membershipId = VALID_UUID;
    form.tenantKey = 'github/acme';
    form.deviceToken = 'tok';
    expect(form.validationError).toBeNull();
    expect(form.canSubmit).toBe(true);
  });

  it('submit() with an invalid draft sets error and never calls connectDojo', async () => {
    const connectDojo = vi.fn();
    const form = new ConnectForm();
    form.membershipId = 'nope';
    const ok = await form.submit(mockApi({ connectDojo }));
    expect(ok).toBe(false);
    expect(form.error).toBe('membership id must be the service membership uuid');
    expect(connectDojo).not.toHaveBeenCalled();
  });

  it('parsedOrgSlugs normalises the raw input from the form field', () => {
    const form = new ConnectForm();
    form.orgSlugs = 'Sensei-HQ, acme  acme';
    expect(form.parsedOrgSlugs).toEqual(['sensei-hq', 'acme']);
  });

  it('submit() posts the built body, resets and closes on success', async () => {
    const connectDojo = vi.fn().mockResolvedValue({ ok: true, data: { id: VALID_UUID } });
    const form = new ConnectForm();
    form.open = true;
    form.membershipId = VALID_UUID;
    form.tenantKey = 'github/acme';
    form.deviceToken = 'tok_123';
    form.kind = 'client';
    form.orgSlugs = 'acme, globex';

    const ok = await form.submit(mockApi({ connectDojo }));
    expect(ok).toBe(true);
    expect(connectDojo).toHaveBeenCalledWith({
      membership_id: VALID_UUID,
      tenant_key: 'github/acme',
      kind: 'client',
      credential: 'tok_123',
      org_slugs: ['acme', 'globex'],
    });
    expect(form.membershipId).toBe('');
    expect(form.deviceToken).toBe('');
    expect(form.orgSlugs).toBe('');
    expect(form.open).toBe(false);
    expect(form.error).toBeNull();
  });

  it('submit() keeps the fields and surfaces the wire error on failure', async () => {
    const connectDojo = vi
      .fn()
      .mockResolvedValue({ ok: false, error: { status: 400, message: 'bad membership_id' } });
    const form = new ConnectForm();
    form.membershipId = VALID_UUID;
    form.tenantKey = 'github/acme';
    form.deviceToken = 'tok_123';

    const ok = await form.submit(mockApi({ connectDojo }));
    expect(ok).toBe(false);
    expect(form.error).toBe('bad membership_id');
    expect(form.membershipId).toBe(VALID_UUID);
    expect(form.submitting).toBe(false);
  });
});
