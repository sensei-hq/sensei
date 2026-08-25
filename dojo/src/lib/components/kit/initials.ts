// Two-letter monogram from a display name — the kit's avatar fallback (the
// window.Avatar treatment). Strips parenthetical qualifiers (e.g. "(Admin)"),
// then takes the first letter of the first two words that *start* with a letter
// or number, uppercased. Shared by TopBar / ChatThread, which each
// carried a byte-identical copy before this was extracted.
//
// Behaviour preserved from the originals: only `null`/`undefined` fall back to
// `fallback`; an empty string yields an empty monogram (callers pass a real name
// or rely on the default).
export function getInitials(name: string | null | undefined, fallback = 'You'): string {
	return (name ?? fallback)
		.replace(/\([^)]*\)/g, ' ')
		.split(/\s+/)
		.filter((w) => /^[\p{L}\p{N}]/u.test(w))
		.slice(0, 2)
		.map((w) => w[0].toUpperCase())
		.join('');
}
