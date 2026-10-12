// The access ladder must rank the tenant roles in the order the schema
// declares them. `dojo.member_role` is `contributor, maintainer, lead, admin`,
// and the canonical entity & access model (docs/architecture/entity-access-model.md)
// says the same: contributor < maintainer < lead < admin.
//
// The ladder here was ported from an older service with LEAD BELOW MAINTAINER
// (2026-07-15). Every floor check then disagreed with the model: a maintainer
// passed the lead floor on client engagements, incidents and the confidentiality
// ledger, and a lead failed the maintainer floor on triage. The route specs stub
// ACCESS with their own copies, so nothing tested the real order.
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { ACCESS, roleToAccess } from './dojo-supabase';

function declaredRoles(): string[] {
	const ddl = readFileSync(
		resolve(__dirname, '../../../../database/ddl/enum/dojo/member_role.ddl'),
		'utf-8'
	);
	const body = ddl.match(/enum\s*\(([^)]*)\)/i);
	if (!body) throw new Error('member_role.ddl declares no enum');
	return [...body[1].matchAll(/'([^']+)'/g)].map((m) => m[1]);
}

describe('the access ladder follows dojo.member_role', () => {
	it('ranks every declared role in the order the schema declares it', () => {
		const roles = declaredRoles();
		expect(roles).toEqual(['contributor', 'maintainer', 'lead', 'admin']);
		const levels = roles.map((r) => roleToAccess(r));
		for (let i = 1; i < levels.length; i++) {
			expect(levels[i], `${roles[i]} must outrank ${roles[i - 1]}`).toBeGreaterThan(levels[i - 1]);
		}
	});

	it('gives no role the member floor — level 0 means no membership', () => {
		for (const r of declaredRoles()) expect(roleToAccess(r)).toBeGreaterThan(ACCESS.member);
		expect(roleToAccess(undefined)).toBe(ACCESS.member);
	});
});
