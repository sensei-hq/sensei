// The statuses a database function's refusals mean (#236). A SECURITY DEFINER
// function that decides access raises a SQLSTATE naming WHY; the caller keeps
// it as that status, with the database's own message. Anything else is a
// failure (500) — never a softer "not found", which would tell a caller that
// something is not theirs when the truth is that the read failed.
//
// Pure (no AdminError import), so admin-data.ts can use it without a cycle.
const STATUS: Record<string, number> = {
	P0002: 404, // no_data_found: not the caller's (or no such thing)
	'55000': 409, // object_not_in_prerequisite_state: not decidable yet
	'42501': 403 // insufficient_privilege: not the caller's to change
};

export function refusalStatus(error: { code?: string }): number {
	return STATUS[error.code ?? ''] ?? 500;
}
