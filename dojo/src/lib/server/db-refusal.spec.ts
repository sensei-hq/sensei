// A database function refuses with a SQLSTATE that names WHY; the route has to
// keep that as the status it means, and never soften a failure into one (#236).
import { describe, it, expect } from 'vitest';
import { refusal } from './db-refusal';
import { AdminError } from './admin-data';

describe('refusal', () => {
	for (const [code, status] of [
		['P0002', 404], // no_data_found
		['55000', 409], // object_not_in_prerequisite_state
		['42501', 403] // insufficient_privilege
	] as const) {
		it(`turns ${code} into ${status}, keeping the database's message`, () => {
			const err = refusal({ code, message: 'why, in the database\'s words' });
			expect(err).toBeInstanceOf(AdminError);
			expect(err.status).toBe(status);
			expect(err.message).toBe("why, in the database's words");
		});
	}
	it('is a 500 for anything else — never a softer "not found"', () => {
		expect(refusal({ code: '08006', message: 'connection reset' }).status).toBe(500);
		expect(refusal({ message: 'no code at all' }).status).toBe(500);
	});
});
