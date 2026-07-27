import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/nav';
import { guardTenantScope } from '$lib/org-guard';
import { listRuns, listGates, getSegments, DojoApiError, type RelayRun, type RelayGate } from '$lib/relay-data';
import { toKitRuns, toKitGates, toKitDecisions, toKitChatThread } from '$lib/relay-map';
import { toKitDojos } from '$lib/chrome';
import {
	projects,
	me,
	stance,
	ladder,
	rulePacks,
	contributionsMine,
	contributionsDownstream,
	contributionsStat
} from '$lib/components/kit/fixtures';

// The personal-zone section loader. The Relay sections (runs · approve · decide ·
// chat) and the memberships list (dojos) now load REAL `/v1` data via the SAME
// console clients the shipped `(console)` relay screens use (relay-data.ts →
// GET /v1/t/{tenant}/relay/…) + the layout's already-loaded memberships, mapped
// to the kit shape by the pure relay-map / chrome mappers. The
// remaining sections (projects · rules · packs · contributions) are still
// presentational off the kit fixtures pending their own routes.
//
// Every tenant-scoped fetch runs behind `guardTenantScope`: a membership-less
// viewer (`tenantKey === null`, DJ1) skips the network entirely and gets an
// honest-empty screen rather than fabricated cross-dōjō work. A live-service
// failure (or a local `/v1` 404 in dev) is caught and degrades to empty lists so
// the page always renders. Your own standing governance (stance · personal
// constitution · adoptable rule packs) is NOT tenant-scoped — it's yours whether
// or not you belong to a dōjō. An unknown section redirects to the landing.

/** The relay lists a personal Relay section needs (runs + all pending gates). */
type RelayLists = { runs: RelayRun[]; gates: RelayGate[] };
const EMPTY_RELAY: RelayLists = { runs: [], gates: [] };

export const load: PageLoad = async ({ params, parent, fetch }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	const { hasMembership, tenantKey, accessToken, memberships } = await parent();
	const section = params.section;

	// Fetch runs + gates only for the sections that render them (runs/approve/
	// decide/chat) and only for a member — the guard short-circuits a solo user.
	const wantsRelay = section === 'runs' || section === 'approve' || section === 'decide' || section === 'chat';
	let relay: RelayLists = EMPTY_RELAY;
	let chatThread: ReturnType<typeof toKitChatThread> = [];
	// The run the /you/chat composer nudges (the newest run — the surface isn't
	// run-scoped in the URL). Null when there's no run to steer.
	let chatRunId: string | null = null;
	let error: string | null = null;
	if (wantsRelay) {
		try {
			const guarded = await guardTenantScope<RelayLists>(tenantKey, EMPTY_RELAY, async (tk) => {
				const [runs, gates] = await Promise.all([
					listRuns(tk, { fetch, accessToken }),
					listGates(tk, { fetch, accessToken })
				]);
				return { runs, gates };
			});
			relay = guarded.value;
			// Chat reads the newest run's outline as the thread (the /you/chat surface
			// isn't run-scoped in the URL — pick the most recent run, if any).
			if (section === 'chat' && tenantKey && relay.runs.length) {
				const newest = relay.runs[0];
				chatRunId = newest.run_id;
				const segments = await getSegments(tenantKey, newest.run_id, { fetch, accessToken });
				chatThread = toKitChatThread(segments);
			}
		} catch (e) {
			error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
		}
	}

	return {
		section,
		title: labelForSection(section, 'you'),
		error,
		// The tenant + token the screen's relay mutations (replyToGate / sendNudge)
		// need — passed through from the layout so the actions can call /v1 directly.
		tenantKey,
		accessToken,
		// Relay screens — real /v1 data mapped to the kit shape (empty when solo/erroring).
		runs: toKitRuns(relay.runs, relay.gates),
		gates: toKitGates(relay.gates),
		decisions: toKitDecisions(relay.gates),
		chat: chatThread,
		chatRunId,
		me,
		// Your own governance — always yours, membership or not (still fixture-backed).
		stance,
		ladder,
		rulePacks,
		// Memberships — real, from the layout's `listUserOrgs`, mapped to KitDojo.
		dojos: toKitDojos(memberships),
		// Still presentational pending their routes.
		projects: hasMembership ? projects : [],
		contributionsMine: hasMembership ? contributionsMine : [],
		contributionsDownstream: hasMembership ? contributionsDownstream : [],
		contributionsStat: hasMembership ? contributionsStat : { approved: 0, pending: 0, helped: 0 }
	};
};
