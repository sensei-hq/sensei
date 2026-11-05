import { describe, it, expect } from 'vitest';
import { toViewerViews, personLabel } from './viewers-view';

// The wire → view mapping for the dōjō's viewers (#307 D8).
describe('viewers view', () => {
	it('maps the wire rows', () => {
		expect(
			toViewerViews([
				{ principal_id: 'v1', display_name: 'Ana', email: 'a@x.co', joined_at: 't', workspaces: [{ id: 'p', name: 'P' }] }
			])
		).toEqual([{ principalId: 'v1', name: 'Ana', email: 'a@x.co', workspaces: [{ id: 'p', name: 'P' }] }]);
	});

	it('labels a viewer by name, else email, else honest words — never an invented name', () => {
		expect(personLabel({ principalId: 'abcdef123456', name: 'Ana', email: 'a@x.co' })).toBe('Ana');
		expect(personLabel({ principalId: 'abcdef123456', name: null, email: 'a@x.co' })).toBe('a@x.co');
		expect(personLabel({ principalId: 'abcdef123456', name: null, email: null })).toBe('Name not shared yet');
	});
	it('L4 member-names — an unnamed person reads as words, never an id fragment', () => {
		expect(personLabel({ principalId: 'abcdef123456', name: null, email: null })).toBe('Name not shared yet');
	});
});
