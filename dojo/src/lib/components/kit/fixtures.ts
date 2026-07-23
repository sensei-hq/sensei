// Test fixtures for the dojo2 kit specs — lifted from the mockup mock data
// (docs/mockups/Sensei/lib/data/dojo2-data.js window.DOJO2) so the specs render
// the components against the exact shapes the screens will bind to.

import type { KitProject, KitDojo, KitMe, KitOrg, KitNavGroup, KitNavItem } from './types';

export const me: KitMe = { name: 'Rin Saito', handle: 'rin', avatar: 'R' };

export const dojos: KitDojo[] = [
	{
		slug: 'acme',
		kanji: '社',
		name: 'Acme Corp',
		kind: 'employer',
		role: 'admin',
		route: 'sensei-hq.com/acme',
		members: 48,
		projects: 9,
		needs: 4
	},
	{
		slug: 'globex',
		kanji: '客',
		name: 'Globex',
		kind: 'client',
		role: 'lead',
		route: 'sensei-hq.com/globex',
		members: 12,
		projects: 3,
		needs: 2
	},
	{
		slug: 'rustco',
		kanji: '群',
		name: 'Rust Guild',
		kind: 'community',
		role: 'developer',
		route: 'sensei-hq.com/rust-guild',
		members: 340,
		projects: 18,
		needs: 0
	}
];

export const org: KitOrg = {
	slug: 'acme',
	kanji: '社',
	name: 'Acme Corp',
	kind: 'employer',
	role: 'admin',
	route: 'sensei-hq.com/acme'
};

export const projects: KitProject[] = [
	{
		id: 'auth',
		name: 'lumen-auth',
		repo: 'acme/lumen-auth',
		dojoName: 'Acme Corp',
		classification: 'company',
		phase: 'notice',
		lastRun: '8m',
		spark: [3, 5, 4, 8, 6, 11, 14],
		needs: 2,
		note: '3 patterns surfacing in payments paths'
	},
	{
		id: 'ledger',
		name: 'ledger-core',
		repo: 'acme/ledger-core',
		dojoName: 'Acme Corp',
		classification: 'company',
		phase: 'adopt',
		lastRun: '2h',
		spark: [6, 7, 5, 9, 8, 9, 9],
		needs: 0,
		note: 'idempotency pattern adopted org-wide'
	},
	{
		id: 'site',
		name: 'personal-site',
		repo: 'rin/personal-site',
		dojoName: null,
		classification: 'personal',
		phase: 'watch',
		lastRun: '1d',
		spark: [2, 1, 3, 2, 4, 2, 3],
		needs: 0,
		note: 'no dōjō · your ladder alone'
	}
];

// A "you"-context nav: personal groups on top (Relay · you / Me), a Work group
// below — the shape the AppShell binds to.
export const nav: KitNavGroup[] = [
	{
		group: 'Relay · you',
		items: [
			{ id: 'today', kanji: '今', label: 'Today', badge: 4 },
			{ id: 'runs', icon: 'eye', label: 'Live runs' }
		]
	},
	{
		group: 'Me',
		items: [
			{ id: 'projects', icon: 'folder', label: 'Projects' },
			{ id: 'contributions', kanji: '共', label: 'Contributions' }
		]
	}
];

export const tabs: KitNavItem[] = [
	{ id: 'today', kanji: '今', label: 'Today', badge: 4 },
	{ id: 'projects', icon: 'folder', label: 'Projects' },
	{ id: 'runs', icon: 'eye', label: 'Runs' }
];
