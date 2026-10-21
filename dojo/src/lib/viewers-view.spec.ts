import { describe, it, expect } from 'vitest';
import { toViewerViews, viewerLabel } from './viewers-view';

// The wire → view mapping for the dōjō's viewers (#307 D8).
describe('viewers view', () => {
	it('maps the wire rows', () => {
		expect(
			toViewerViews([
				{ principal_id: 'v1', display_name: 'Ana', email: 'a@x.co', joined_at: 't', projects: [{ id: 'p', name: 'P' }] }
			])
		).toEqual([{ principalId: 'v1', name: 'Ana', email: 'a@x.co', projects: [{ id: 'p', name: 'P' }] }]);
	});

	it('labels a viewer by name, else email, else a short id — never an invented name', () => {
		expect(viewerLabel({ principalId: 'abcdef123456', name: 'Ana', email: 'a@x.co', projects: [] })).toBe('Ana');
		expect(viewerLabel({ principalId: 'abcdef123456', name: null, email: 'a@x.co', projects: [] })).toBe('a@x.co');
		expect(viewerLabel({ principalId: 'abcdef123456', name: null, email: null, projects: [] })).toBe('abcdef12');
	});
});
