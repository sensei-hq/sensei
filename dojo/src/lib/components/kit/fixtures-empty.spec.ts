import { describe, it, expect } from 'vitest';
import {
	orgProjectsFor,
	orgConstitutionFor,
	triageGroupsFor,
	approvalsFor,
	knowledgeFor,
	engagementsFor,
	incidentsFor,
	clientAuditFor,
	scopeOwnersFor
} from './fixtures';

// An unknown org slug must render honestly empty — never another org's (Acme's)
// data. The mockup's `x[slug] ?? x.acme` fallback was removed so a real org route
// shows an empty state until its own /v1 data is wired.
describe('org fixtures — an unknown slug returns empty, never Acme', () => {
	const unknown = 'no-such-org';

	it('array-returning lookups are empty for an unknown slug', () => {
		expect(orgProjectsFor(unknown)).toEqual([]);
		expect(orgConstitutionFor(unknown)).toEqual([]);
		expect(triageGroupsFor(unknown)).toEqual([]);
		expect(approvalsFor(unknown)).toEqual([]);
		expect(engagementsFor(unknown)).toEqual([]);
		expect(incidentsFor(unknown)).toEqual([]);
		expect(clientAuditFor(unknown)).toEqual([]);
		expect(scopeOwnersFor(unknown)).toEqual([]);
	});

	it('knowledge is an empty shape (no Acme rows) for an unknown slug', () => {
		expect(knowledgeFor(unknown)).toEqual({
			prunePolicy: '',
			active: [],
			pending: [],
			catalog: []
		});
	});
});
