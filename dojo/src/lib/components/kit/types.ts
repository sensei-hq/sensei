// Shapes the dojo2 kit rows bind to. Ported from the mockup fixtures
// (docs/mockups/Sensei/lib/data/dojo2-data.js) so screens and specs share one
// self-describing contract. Kept flat and optional-heavy to match the mock; the
// wire types will refine these when the screens land.

/** A project in flight — the row workhorse (dojo2-data `projects[]`). */
export interface KitProject {
	id: string;
	name: string;
	repo: string;
	/** company · client · personal · community. */
	classification: string;
	/** watch · notice · adopt. */
	phase: string;
	/** Compact "how long ago" the last run was (e.g. "8m"). */
	lastRun?: string;
	/** Count of things in this project that need the viewer; 0 = none. */
	needs?: number;
	/** Owning dōjō display name (shown when `showDojo`). */
	dojoName?: string | null;
	/** A short human note about the project's current signal. */
	note?: string | null;
	/** Sparkline series for the recent run cadence. */
	spark?: number[];
}

/** A dōjō membership row (dojo2-data `dojos[]`). */
export interface KitDojo {
	slug: string;
	kanji: string;
	name: string;
	/** employer · client · community · personal. */
	kind: string;
	/** developer · maintainer · lead · admin. */
	role: string;
	/** The dōjō's public route (e.g. "sensei-hq.com/acme"). */
	route: string;
	members: number;
	projects: number;
	/** Count of things in this dōjō that need the viewer; 0 = none. */
	needs?: number;
}

/** The viewer (dojo2-data `me`). */
export interface KitMe {
	name: string;
	handle?: string;
	avatar?: string;
}

/** An org context — a dōjō the viewer has stepped into (dojo2-data `dojos[]`). */
export interface KitOrg {
	slug: string;
	kanji: string;
	name: string;
	kind: string;
	role: string;
	route: string;
}

/** A nav item inside a NavPane group or a TabBar (icon OR kanji + label). */
export interface KitNavItem {
	id: string;
	label: string;
	icon?: string;
	kanji?: string;
	/** Optional count badge. */
	badge?: number;
}

/** A titled group of nav items in the NavPane. */
export interface KitNavGroup {
	group?: string;
	items: KitNavItem[];
}
