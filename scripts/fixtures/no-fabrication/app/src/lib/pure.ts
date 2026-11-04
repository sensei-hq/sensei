// Fixture: no fetch in this file, so `?? []` is an optional-field default (not T3).
export function names(list?: string[]) {
	const all = list ?? [];
	const n = all.length || 0;
	return { all, n };
}
