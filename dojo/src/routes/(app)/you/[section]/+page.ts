import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/nav';
import { toKitDojos } from '$lib/chrome';
import { listUserProjects, listContributions, listLibraryPacks, ClientApiError } from '$lib/client-data';
import { toKitProjects } from '$lib/projects-map';
import { toKitContributions, toKitDownstreams } from '$lib/contributions-map';
import { toKitRulePacks } from '$lib/rulepacks-map';
import type { KitProject, KitContribution, KitDownstream, KitRulePack } from '$lib/components/kit/types';
import { stance, ladder } from '$lib/components/kit/fixtures';

// The personal-zone section loader — the non-inbox destinations (projects ·
// rules · packs · dojos · contributions). `dojos` binds REAL memberships (from
// the layout); `projects` binds the REAL user-wide `dojo.projects` read (F1,
// GET /v1/you/projects) — honest-empty on a genuine empty, an error STATE on a
// read failure (never []-as-success masking a failure). Contributions still has
// no backing route → honest empty (not a fabricated "helped 612"). Your own
// governance (stance · constitution · rule packs) stays fixture-backed pending
// its own route. An unknown/retired section redirects to the Inbox landing.
export const load: PageLoad = async ({ params, parent, fetch }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { memberships, accessToken } = await parent();
	const section = params.section;

	// Real projects — only fetched on the projects section (no needless /v1 call
	// on the fixture-backed sections). The read is user-wide (across every dōjō).
	let projects: KitProject[] = [];
	let projectsError: string | null = null;
	if (section === 'projects') {
		try {
			projects = toKitProjects(await listUserProjects({ fetch, accessToken }));
		} catch (e) {
			projectsError = e instanceof ClientApiError ? e.message : 'could not reach the dojo service';
		}
	}

	// Real contributions (F5) — user-wide across every dōjō, only on this section.
	// Honest-empty until the contribute pipeline federates; a transient read error
	// degrades to empty (a read-only informational surface).
	let contributionsMine: KitContribution[] = [];
	let contributionsDownstream: KitDownstream[] = [];
	if (section === 'contributions') {
		try {
			const c = await listContributions({ fetch, accessToken });
			contributionsMine = toKitContributions(c.mine);
			contributionsDownstream = toKitDownstreams(c.downstream);
		} catch {
			// honest-empty — the sections render their empty state.
		}
	}

	// Real rule-pack library (browse) — the user-wide global catalog (GET
	// /v1/you/rule-packs), only on this section. Honest-empty on a read failure
	// (read-only informational surface), never the old fixture.
	let rulePacks: KitRulePack[] = [];
	if (section === 'packs') {
		try {
			const { packs, adopted } = await listLibraryPacks({ fetch, accessToken });
			rulePacks = toKitRulePacks(packs, new Set(adopted));
		} catch {
			// honest-empty — the screen renders its empty state.
		}
	}

	return {
		section,
		title: labelForSection(section, 'you'),
		// Your own governance — stance/constitution still fixture-backed pending their routes.
		stance,
		ladder,
		// Rule packs — real global library read (browse); honest-empty on failure.
		rulePacks,
		// Memberships — real, from the layout's `listUserOrgs`, mapped to KitDojo.
		dojos: toKitDojos(memberships),
		// Projects — real user-wide read; error ≠ empty (F1).
		projects,
		projectsError,
		// Contributions — real user-wide read (F5); honest-empty until federated.
		contributionsMine,
		contributionsDownstream
	};
};
