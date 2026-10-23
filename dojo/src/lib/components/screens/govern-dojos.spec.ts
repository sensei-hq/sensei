import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrConstitution from './ScrConstitution.svelte';
import ScrRulePacks from './ScrRulePacks.svelte';
import ScrMyDojos from './ScrMyDojos.svelte';
import ScrContributions from './ScrContributions.svelte';
import {
	stance,
	ladder,
	personalLadder,
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
		const { getByText, getAllByText } = render(ScrConstitution, { props: { stance, ladder: personalLadder } });
		expect(getByText('Your constitution')).toBeTruthy();
		// one dial per stance axis (autonomy · sharing · review).
		expect(getByText('autonomy')).toBeTruthy();
		expect(getByText('sharing')).toBeTruthy();
		expect(getByText('review')).toBeTruthy();
		// your personal rung renders — the rules that stay yours, in sensei (D25).
		expect(getAllByText('Personal').length).toBeGreaterThan(0);
	});

	it('omits the dōjō ladder rungs (dōjō · workspace · repository) from a full composed ladder', () => {
		const { queryByText, getAllByText } = render(ScrConstitution, {
			props: { stance, ladder: [...personalLadder, ...ladder] }
		});
		expect(getAllByText('Personal').length).toBeGreaterThan(0);
		expect(queryByText('Dōjō')).toBeNull();
		expect(queryByText('Repository')).toBeNull();
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

	// #293: the flip is optimistic, so a refused adopt must flip back and say why.
	it('puts a refused adopt back and shows the reason', async () => {
		const onToggle = vi.fn().mockResolvedValue(false);
		const available = rulePacks.length - rulePacks.filter((p) => p.adopted).length;
		const { getAllByText, rerender, getByText } = render(ScrRulePacks, { props: { packs: rulePacks, onToggle } });
		await fireEvent.click(getAllByText('Adopt')[0]);
		await vi.waitFor(() => expect(getAllByText('Adopt').length).toBe(available));
		await rerender({ packs: rulePacks, onToggle, error: 'Not adopted: forbidden' });
		expect(getByText('Not adopted: forbidden')).toBeTruthy();
	});

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

	it('with no memberships, says where a dōjō comes from — none is made by hand (#307 D11)', () => {
		const { getByText, queryByText } = render(ScrMyDojos, { props: { dojos: [] } });
		expect(getByText('No dōjōs yet.')).toBeTruthy();
		expect(getByText(/your GitHub organizations, or an invite/)).toBeTruthy();
		expect(queryByText('Organization')).toBeNull();
		expect(queryByText(/Create/)).toBeNull();
	});

	it('offers Sync from GitHub in its own header, so the route renders only the screen (#293)', async () => {
		const onSyncGithub = vi.fn();
		const { getByText, container } = render(ScrMyDojos, { props: { dojos, onSyncGithub } });
		const header = container.querySelector('[data-component="screen-header"]')!;
		const button = getByText('Sync from GitHub');
		expect(header.contains(button)).toBe(true);
		await fireEvent.click(button);
		expect(onSyncGithub).toHaveBeenCalled();
	});

	it('shows the sync in flight and what it came to', () => {
		const { getByText } = render(ScrMyDojos, {
			props: { dojos, onSyncGithub: vi.fn(), syncing: true, syncMessage: '2 dōjōs synced' }
		});
		expect((getByText('Syncing…').closest('button') as HTMLButtonElement).disabled).toBe(true);
		expect(getByText('2 dōjōs synced')).toBeTruthy();
	});

	it('offers no sync without a handler', () => {
		const { queryByText } = render(ScrMyDojos, { props: { dojos } });
		expect(queryByText('Sync from GitHub')).toBeNull();
	});

	it('offers no way to make a dōjō by hand (#307 D11)', () => {
		const { queryByText } = render(ScrMyDojos, { props: { dojos, onSyncGithub: vi.fn() } });
		expect(queryByText(/Create/)).toBeNull();
	});
});

describe('ScrContributions — shared upstream + approved downstream', () => {
	afterEach(cleanup);

	it('shows why a pin failed', () => {
		const { getByText } = render(ScrContributions, {
			props: { mine: contributionsMine, downstream: contributionsDownstream, stat: contributionsStat, error: 'Not pinned: forbidden' }
		});
		expect(getByText('Not pinned: forbidden')).toBeTruthy();
	});

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
