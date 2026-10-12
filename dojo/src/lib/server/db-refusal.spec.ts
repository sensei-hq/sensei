// A database function refuses with a SQLSTATE that names WHY; the route has to
// keep that as the status it means, and never soften a failure into one (#236).
import { describe, it, expect } from 'vitest';
import { refusalStatus } from './db-refusal';

describe('refusalStatus', () => {
	for (const [code, status] of [
		['P0002', 404], // no_data_found
		['55000', 409], // object_not_in_prerequisite_state
		['42501', 403] // insufficient_privilege
	] as const) {
		it(`turns ${code} into ${status}`, () => {
			expect(refusalStatus({ code })).toBe(status);
		});
	}
	it('is a 500 for anything else — never a softer "not found"', () => {
		expect(refusalStatus({ code: '08006' })).toBe(500);
		expect(refusalStatus({})).toBe(500);
	});
});
