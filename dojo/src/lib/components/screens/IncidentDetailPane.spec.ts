import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import IncidentDetailPane from './IncidentDetailPane.svelte';
import type { KitIncidentDetail } from '$lib/components/kit/types';

// The incident "Open" detail pane. Presentational — renders a KitIncidentDetail
// (the resolved owner / SLA / resolution / linked artifact), or nothing when null.

const detail: KitIncidentDetail = {
	id: 'i1',
	title: 'Near-leak: client hostname in a shared prompt',
	client: 'Globex',
	owner: 'Ada Lovelace',
	state: 'contained',
	severity: 'high',
	opened: '3d',
	sla: '2026-08-01',
	resolution: 'contained — source stripped before it left',
	artifact: { title: 'the auth pattern', kind: 'pattern', status: 'archived' }
};

describe('IncidentDetailPane', () => {
	afterEach(cleanup);

	it('renders nothing when detail is null', () => {
		const { container } = render(IncidentDetailPane, { props: { detail: null } });
		expect(container.textContent?.trim()).toBe('');
	});

	it('renders the title, resolved owner, SLA, resolution, and linked artifact', () => {
		const { container } = render(IncidentDetailPane, { props: { detail } });
		const t = container.textContent ?? '';
		expect(t).toMatch(/Near-leak/);
		expect(t).toMatch(/Ada Lovelace/);
		expect(t).toMatch(/2026-08-01/);
		expect(t).toMatch(/source stripped/);
		expect(t).toMatch(/the auth pattern/);
	});

	it('omits the SLA / resolution / artifact rows when absent', () => {
		const bare: KitIncidentDetail = {
			...detail,
			owner: '—',
			sla: null,
			resolution: null,
			artifact: null
		};
		const { container } = render(IncidentDetailPane, { props: { detail: bare } });
		const t = container.textContent ?? '';
		expect(t).not.toMatch(/SLA due/);
		expect(t).not.toMatch(/the auth pattern/);
		expect(t).toMatch(/Owner · —/);
	});

	it('renders a Close button that fires onClose when provided', async () => {
		let closed = false;
		const { getByText } = render(IncidentDetailPane, {
			props: { detail, onClose: () => (closed = true) }
		});
		const btn = getByText('Close');
		expect(btn).toBeTruthy();
		btn.closest('button')?.click();
		expect(closed).toBe(true);
	});

	it('matches snapshot', () => {
		const { container } = render(IncidentDetailPane, { props: { detail } });
		expect(container).toMatchSnapshot();
	});
});
