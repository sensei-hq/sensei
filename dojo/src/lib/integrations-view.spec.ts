import { describe, it, expect } from 'vitest';
import { dojoLine, mappingLine, type IntegrationUnit } from './integrations-view';

// Admin → Integrations (#308 R5a): each GitHub org says what its dōjō is and
// where its repositories file — in words, from the wire row alone. An unclaimed
// dōjō says who can claim it.
const base: IntegrationUnit = {
	provider: 'github',
	id: '22',
	login: 'acme',
	role: 'member',
	dojo: null,
	mapping: null,
	canClaim: false,
	canMapAsDojo: false,
	canMapAsWorkspace: true
};

describe('dojoLine', () => {
	it('says when the org has no dōjō yet', () => {
		expect(dojoLine(base)).toBe('No dōjō yet');
	});
	it('says an unclaimed dōjō can be claimed by an admin of the org on GitHub', () => {
		const u = { ...base, dojo: { key: 'organization/acme', name: 'acme', claimed: false, claimedBy: null } };
		expect(dojoLine(u)).toBe('organization/acme · unclaimed — an admin of acme on GitHub can claim it');
	});
	it('names who claimed it, or says only that it is claimed', () => {
		const claimed = { key: 'organization/acme', name: 'acme', claimed: true, claimedBy: 'Ada' };
		expect(dojoLine({ ...base, dojo: claimed })).toBe('organization/acme · claimed by Ada');
		expect(dojoLine({ ...base, dojo: { ...claimed, claimedBy: null } })).toBe('organization/acme · claimed');
	});
});

describe('mappingLine', () => {
	it('an unmapped org files into its own dōjō when it has one, nowhere when it has none', () => {
		expect(mappingLine({ ...base, dojo: { key: 'organization/acme', name: 'acme', claimed: false, claimedBy: null } })).toBe(
			'Repositories file into its own dōjō'
		);
		expect(mappingLine(base)).toBe('Repositories file nowhere yet');
	});
	it('names a workspace here, and another dōjō only when the person may know it', () => {
		expect(mappingLine({ ...base, mapping: { as: 'dojo' } })).toBe('Repositories file into its own dōjō');
		expect(
			mappingLine({ ...base, mapping: { as: 'workspace', inThisDojo: true, dojoKey: 'organization/here', workspaceName: 'Acme' } })
		).toBe('Repositories file into the workspace Acme in this dōjō');
		expect(
			mappingLine({ ...base, mapping: { as: 'workspace', inThisDojo: false, dojoKey: 'organization/other', workspaceName: 'Acme' } })
		).toBe('Repositories file into the workspace Acme in organization/other');
		expect(
			mappingLine({ ...base, mapping: { as: 'workspace', inThisDojo: false, dojoKey: null, workspaceName: null } })
		).toBe('Repositories file into a workspace of another dōjō');
	});
});
