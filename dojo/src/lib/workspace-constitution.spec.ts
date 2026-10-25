import { describe, it, expect, vi } from 'vitest';
import { loadWorkspaceConstitution } from './workspace-constitution';

// One workspace's daemon-resolved constitution, for both preview drill-ins (the
// personal and the org one, #106). A failed read is an error the screen shows —
// it used to be swallowed into "resolves in your editor", a failure shown as
// nothing federated yet.
function respond(status: number, body: unknown): typeof fetch {
	return vi.fn(async () => ({ ok: status < 300, status, json: async () => body }) as Response) as unknown as typeof fetch;
}

describe('loadWorkspaceConstitution', () => {
	it('maps a federated constitution to the ladder and its conflicts', async () => {
		const constitution = {
			rules: [{ scope_key: 'organization', namespace: 'sensei-hq', title: 'No secrets in source', enforcement: 'mandatory' }],
			conflicts: [{ topic: 'retries', loser_scope: 'organization', winner_scope: 'workspace', why: 'narrower wins', locked: false }],
			locks: 0
		};
		const out = await loadWorkspaceConstitution('w1', { fetch: respond(200, { constitution }) });
		expect(out.error).toBeNull();
		expect(out.ladder.flatMap((r) => (r.rules ?? []).map((x) => x.text))).toEqual(['No secrets in source']);
		expect(out.conflicts.map((c) => c.topic)).toEqual(['retries']);
	});

	it('nothing federated yet is empty, not an error', async () => {
		const out = await loadWorkspaceConstitution('w1', { fetch: respond(200, { constitution: null }) });
		expect(out).toEqual({ ladder: [], conflicts: [], error: null });
	});

	it('a failed read is an error, never an empty constitution', async () => {
		const out = await loadWorkspaceConstitution('w1', { fetch: respond(500, { error: 'boom' }) });
		expect(out.ladder).toEqual([]);
		expect(out.error).toBeTruthy();
	});
});
