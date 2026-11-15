// Test fixtures for Settings · Dōjō connection — imported by its specs only.
import type { DojoMembership, SyncStateRow } from '$lib/types.js';
import type { Identity } from '$lib/personas.svelte.js';

export function identity(over: Partial<Identity> = {}): Identity {
  return {
    label: 'jerrythomas', name: 'jerrythomas', githubLogin: 'jerrythomas', sessionSlot: 'jerrythomas',
    isDefault: true, status: 'ready', expiresAt: null, standing: '6.5 hours left', tone: 'ok',
    action: null, busy: false, buttonLabel: null, badge: 'ready', ...over,
  };
}

export function membership(over: Partial<DojoMembership> = {}): DojoMembership {
  return {
    id: 'm1', registry_url: 'https://dojo.sensei-hq.com', tenant_key: 'github/sensei-hq',
    dojo_url: 'https://dojo.sensei-hq.com/github/sensei-hq', kind: 'employer', role: 'contributor',
    authenticated_via: 'github_oauth', attribution_default: 'named', org_slugs: ['sensei-hq'],
    sync_status: 'authenticating', last_seq: 0, last_heartbeat_at: null, enabled: true,
    bound_workspaces: [], persona_slot: 'jerrythomas', ...over,
  };
}

export function repo(over: Partial<SyncStateRow> = {}): SyncStateRow {
  return {
    entity: 'repository', entity_key: 'github.com/sensei-hq/sensei', direction: 'push', state: 'synced',
    last_error: null, attempted_at: '2026-11-02T10:00:00Z', synced_at: '2026-11-02T10:00:00Z',
    updated_at: '2026-11-02T10:00:00Z', persona: 'jerrythomas', remedy: null, ...over,
  };
}
