import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/nav';
import { toKitDojos } from '$lib/chrome';
import {
	listUserWorkspaces,
	listContributions,
	listLibraryPacks,
	listMyRepos,
	getMyMetrics,
	getMicrosoftLink,
	ClientApiError,
	type MyRepoWire
} from '$lib/client-data';
import { toKitWorkspaces } from '$lib/workspaces-map';
import { toKitContributions, toKitDownstreams } from '$lib/contributions-map';
import { toKitRulePacks } from '$lib/rulepacks-map';
import { microsoftNotice as noticeFor } from '$lib/azure-view';
import { toMyMetricsView, type MyMetricsView } from '$lib/my-metrics-view';
import type { KitWorkspace, KitContribution, KitDownstream, KitRulePack } from '$lib/components/kit/types';

// The personal-zone section loader — the non-inbox destinations (workspaces ·
// rules · packs · dojos · contributions). `dojos` binds REAL memberships (from
// the layout); `workspaces` binds the REAL user-wide `dojo.workspaces` read (F1,
// GET /v1/you/workspaces) — honest-empty on a genuine empty, an error STATE on a
// read failure (never []-as-success masking a failure). Contributions still has
// no backing route → honest empty (not a fabricated "helped 612"). Your stance
// and personal rules are not the dōjō's: sensei keeps them on your machine (D25),
// so the rules section loads none and says so (#106). An unknown/retired section redirects to the Inbox landing.
export const load: PageLoad = async ({ params, parent, fetch }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { memberships, accessToken } = await parent();
	const section = params.section;

	// Real workspaces — only fetched on the workspaces section (no needless /v1 call
	// on the fixture-backed sections). The read is user-wide (across every dōjō).
	let workspaces: KitWorkspace[] = [];
	let workspacesError: string | null = null;
	if (section === 'workspaces') {
		try {
			workspaces = toKitWorkspaces(await listUserWorkspaces({ fetch, accessToken }));
		} catch (e) {
			workspacesError = e instanceof ClientApiError ? e.message : 'could not reach the dojo service';
		}
	}

	// Real contributions (F5) — user-wide across every dōjō, only on this section.
	// Empty until the contribute pipeline federates; a failed read is carried as
	// an error, never as "nothing shared" (#109).
	let contributionsMine: KitContribution[] = [];
	let contributionsDownstream: KitDownstream[] = [];
	let contributionsError: string | null = null;
	if (section === 'contributions') {
		try {
			const c = await listContributions({ fetch, accessToken });
			contributionsMine = toKitContributions(c.mine);
			contributionsDownstream = toKitDownstreams(c.downstream);
		} catch (e) {
			contributionsError = e instanceof Error ? e.message : 'the request failed';
		}
	}

	// Real rule-pack library (browse) — the user-wide global catalog (GET
	// /v1/you/rule-packs), only on this section. A failed read is carried as an
	// error, never as an empty library (#109).
	let rulePacks: KitRulePack[] = [];
	let packsError: string | null = null;
	if (section === 'packs') {
		try {
			const { packs, adopted } = await listLibraryPacks({ fetch, accessToken });
			rulePacks = toKitRulePacks(packs, new Set(adopted));
		} catch (e) {
			packsError = e instanceof Error ? e.message : 'the request failed';
		}
	}

	// Sharing — the decision surface. `reposError` is carried SEPARATELY from an
	// empty list on purpose: "you have no repositories" and "we could not ask"
	// are different answers, and collapsing them is what makes a broken read look
	// like a settled state (F1, and the #109 fabrication audit).
	let repos: MyRepoWire[] = [];
	let reposError: string | null = null;
	if (section === 'sharing') {
		try {
			repos = await listMyRepos({ fetch, accessToken });
		} catch (e) {
			reposError = e instanceof Error ? e.message : 'the request failed';
		}
	}

	// My metrics — the caller's own series. A failure is carried SEPARATELY from an
	// empty month, like the Sharing read above: "nothing sent" and "could not ask"
	// are different answers.
	let metricsView: MyMetricsView | null = null;
	let metricsError: string | null = null;
	if (section === 'metrics') {
		try {
			metricsView = toMyMetricsView(await getMyMetrics({ fetch, accessToken }));
		} catch (e) {
			metricsError = e instanceof Error ? e.message : 'the request failed';
		}
	}

	// Whether the Microsoft account is linked (#308 R5c) — said on My dōjōs only
	// when it could not be. A failed check is said too, never read as "linked".
	let microsoftNotice: string | null = null;
	if (section === 'dojos') {
		try {
			microsoftNotice = noticeFor(await getMicrosoftLink({ fetch, accessToken }));
		} catch (e) {
			microsoftNotice = `Could not check your Microsoft account link: ${e instanceof Error ? e.message : 'the request failed'}.`;
		}
	}

	return {
		section,
		microsoftNotice,
		repos,
		reposError,
		metricsView,
		metricsError,
		title: labelForSection(section, 'you'),
		// Rule packs — real global library read (browse); a failure is an error.
		rulePacks,
		packsError,
		// Memberships — real, from the layout's `listUserOrgs`, mapped to KitDojo.
		dojos: toKitDojos(memberships),
		// Workspaces — real user-wide read; error ≠ empty (F1).
		workspaces,
		workspacesError,
		// Contributions — real user-wide read (F5); empty until federated, and a
		// failure is an error.
		contributionsMine,
		contributionsDownstream,
		contributionsError
	};
};
