import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrConstitution from './ScrConstitution.svelte';
import ScrRulePacks from './ScrRulePacks.svelte';
import ScrMyDojos from './ScrMyDojos.svelte';
import ScrContributions from './ScrContributions.svelte';
import {
	stance,
	ladder,
	rulePacks,
	dojos,
	contributionsMine,
	contributionsDownstream,
	contributionsStat
} from '$lib/components/kit/fixtures';

// Chunk-3 personal Govern + Dōjōs screens (completing NAV_YOU). Each renders off
// the kit fixtures (presentational — real /v1 wiring is a later chunk). We
// assert the stance dials render + change, the packs adopt/drop toggle moves a
// row between sections, memberships group employer/clients/communities + the
// honest empty state, and the contributions status tallies + rows render.

describe('ScrConstitution — stance dials + personal ladder', () => {
	afterEach(cleanup);

	it('renders a stance dial per axis and the personal-constitution rungs', () => {
		const { getByText, getAllByText } = render(ScrConstitution, { props: { stance, ladder } });
		expect(getByText('Your constitution')).toBeTruthy();
		// one dial per stance axis (autonomy · sharing · review).
		expect(getByText('autonomy')).toBeTruthy();
		expect(getByText('sharing')).toBeTruthy();
		expect(getByText('review')).toBeTruthy();
		// the effective (personal + stack) rungs render — not the org-scoped ones.
		expect(getAllByText('Personal').length).toBeGreaterThan(0);
		expect(getAllByText('Stack').length).toBeGreaterThan(0);
	});

	it('omits the org-scoped rungs (company · client)', () => {
		const { queryByText } = render(ScrConstitution, { props: { stance, ladder } });
		expect(queryByText('Company')).toBeNull();
		expect(queryByText('Client')).toBeNull();
	});

	it('fires onStance when a dial dot is picked', async () => {
		const onStance = vi.fn();
		const { getByLabelText } = render(ScrConstitution, { props: { stance, ladder, onStance } });
		// the autonomy dial's top level ("run freely").
		await fireEvent.click(getByLabelText('run freely'));
		expect(onStance).toHaveBeenCalledWith('autonomy', 3);
	});

	it('fires onGoPacks from the rule-packs link', async () => {
		const onGoPacks = vi.fn();
		const { getByText } = render(ScrConstitution, { props: { stance, ladder, onGoPacks } });
		await fireEvent.click(getByText('Rule packs →'));
		expect(onGoPacks).toHaveBeenCalled();
	});
});

describe('ScrRulePacks — adopt/drop rule bundles', () => {
	afterEach(cleanup);

	it('splits packs into adopted and available sections', () => {
		const { getByText } = render(ScrRulePacks, { props: { packs: rulePacks } });
		expect(getByText('Rule packs')).toBeTruthy();
		expect(getByText('Adopted')).toBeTruthy();
		expect(getByText('Available')).toBeTruthy();
		// an adopted pack + an available pack both render by name.
		expect(getByText('OWASP ASVS · application security')).toBeTruthy();
		expect(getByText('Svelte 5 · runes')).toBeTruthy();
	});

	it('adopting an available pack toggles it to adopted (and fires onToggle)', async () => {
		const onToggle = vi.fn();
		const { getAllByText, queryAllByText } = render(ScrRulePacks, {
			props: { packs: rulePacks, onToggle }
		});
		// the fixture split drives the Adopt / Drop tallies (adopted → Drop,
		// available → Adopt) — derived so the credible-pack content can grow.
		const adopted = rulePacks.filter((p) => p.adopted).length;
		const available = rulePacks.length - adopted;
		expect(getAllByText('Adopt').length).toBe(available);
		expect(getAllByText('Drop').length).toBe(adopted);
		await fireEvent.click(getAllByText('Adopt')[0]);
		expect(onToggle).toHaveBeenCalled();
		// the toggled pack moved: one fewer Adopt, one more Drop.
		expect(queryAllByText('Adopt').length).toBe(available - 1);
		expect(getAllByText('Drop').length).toBe(adopted + 1);
	});

	it('shows each pack rule count as a disclosure, collapsed by default', () => {
		const { getAllByLabelText, queryByText } = render(ScrRulePacks, { props: { packs: rulePacks } });
		// a disclosure per pack (one "Show rules" toggle each), collapsed → the
		// rule text is not in the DOM yet.
		expect(getAllByLabelText('Show rules').length).toBe(rulePacks.length);
		const sample = rulePacks[0].rules[0];
		expect(queryByText(sample)).toBeNull();
	});

	it('expanding a pack reveals its rules (and flips aria-expanded)', async () => {
		const target = rulePacks.find((p) => p.id === 'conventional')!;
		const { getByText, getAllByLabelText, queryByText } = render(ScrRulePacks, {
			props: { packs: rulePacks }
		});
		// the pack sits in the Available section (not adopted) — find its row by
		// name, then its sibling disclosure toggle.
		const nameEl = getByText(target.name);
		const toggle = nameEl.parentElement!.querySelector('button[aria-expanded]') as HTMLButtonElement;
		expect(toggle.getAttribute('aria-expanded')).toBe('false');
		// before expanding, the pack's rule text is absent.
		expect(queryByText(target.rules[0])).toBeNull();
		await fireEvent.click(toggle);
		expect(toggle.getAttribute('aria-expanded')).toBe('true');
		// after expanding, every rule statement in the pack renders.
		for (const rule of target.rules) {
			expect(getByText(rule)).toBeTruthy();
		}
		// the disclosure now offers to hide the rules again.
		expect(getAllByLabelText('Hide rules').length).toBe(1);
	});
});

describe('ScrMyDojos — memberships grouped + empty state', () => {
	afterEach(cleanup);

	it('groups memberships under Organizations, with no relationship buckets', () => {
		const { getByText, queryByText } = render(ScrMyDojos, { props: { dojos } });
		expect(getByText('My dōjōs')).toBeTruthy();
		expect(getByText('Organizations')).toBeTruthy();
		// a membership renders by name.
		expect(getByText('Acme Corp')).toBeTruthy();
		// The buckets that asserted a relationship nobody stated are gone from the
		// screen, not merely relabelled.
		for (const gone of ['Employer', 'Clients', 'Communities']) {
			expect(queryByText(gone)).toBeNull();
		}
	});

	it('fires onOpen when a membership row is clicked', async () => {
		const onOpen = vi.fn();
		const { getByText } = render(ScrMyDojos, { props: { dojos, onOpen } });
		await fireEvent.click(getByText('Acme Corp'));
		expect(onOpen).toHaveBeenCalledWith(dojos.find((d) => d.name === 'Acme Corp'));
	});

	it('shows the honest empty state with no memberships', () => {
		const onCreateOrJoin = vi.fn();
		const { getByText, queryByText } = render(ScrMyDojos, {
			props: { dojos: [], onCreateOrJoin }
		});
		expect(getByText('No memberships yet — create or join a Dōjō.')).toBeTruthy();
		expect(queryByText('Organization')).toBeNull();
	});

	it('fires onCreateOrJoin from the empty-state affordance', async () => {
		const onCreateOrJoin = vi.fn();
		const { getByText } = render(ScrMyDojos, { props: { dojos: [], onCreateOrJoin } });
		await fireEvent.click(getByText('Create or join a dōjō'));
		expect(onCreateOrJoin).toHaveBeenCalled();
	});
});

describe('ScrContributions — shared upstream + approved downstream', () => {
	afterEach(cleanup);

	it('tallies the stat row and renders both sections', () => {
		const { getByText } = render(ScrContributions, {
			props: {
				mine: contributionsMine,
				downstream: contributionsDownstream,
				stat: contributionsStat
			}
		});
		expect(getByText('Contributions')).toBeTruthy();
		expect(getByText("What you've shared")).toBeTruthy();
		expect(getByText('Approved for you')).toBeTruthy();
		// an upstream row and a downstream row render by title.
		expect(getByText('Adapter wraps a third-party SDK behind a trait')).toBeTruthy();
		expect(getByText('Never log refresh tokens, even at debug level')).toBeTruthy();
	});

	it('shows the per-destination status chips (approved · in triage · declined)', () => {
		const { getByText, getAllByText } = render(ScrContributions, {
			props: {
				mine: contributionsMine,
				downstream: contributionsDownstream,
				stat: contributionsStat
			}
		});
		// "in triage" is both the pending stat-badge label and a pending row chip.
		expect(getAllByText('in triage').length).toBeGreaterThan(1);
		expect(getByText('declined')).toBeTruthy();
	});

	it('fires onPin for an unadopted downstream item', async () => {
		const onPin = vi.fn();
		const { getAllByText } = render(ScrContributions, {
			props: {
				mine: contributionsMine,
				downstream: contributionsDownstream,
				stat: contributionsStat,
				onPin
			}
		});
		// two downstream items are unadopted → two Pin buttons; click the first.
		await fireEvent.click(getAllByText('Pin')[0]);
		expect(onPin).toHaveBeenCalledWith(contributionsDownstream.find((d) => !d.adopted));
	});

	it('is empty-safe (no rows, zeroed tally)', () => {
		const { getByText, queryByText } = render(ScrContributions, {
			props: { mine: [], downstream: [], stat: { approved: 0, pending: 0, helped: 0 } }
		});
		expect(getByText('Contributions')).toBeTruthy();
		expect(queryByText('Adapter wraps a third-party SDK behind a trait')).toBeNull();
	});
});
