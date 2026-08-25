import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import Icon from './Icon.svelte';
import KanjiToken from './KanjiToken.svelte';
import ClassChip from './ClassChip.svelte';
import RoleTag from './RoleTag.svelte';
import PhasePill from './PhasePill.svelte';
import StatBadge from './StatBadge.svelte';
import ProjectRow from './ProjectRow.svelte';
import MyDojoRow from './MyDojoRow.svelte';
import ConfidenceBar from './ConfidenceBar.svelte';
import Spark from './Spark.svelte';
import Enso from './Enso.svelte';
import { projects, dojos } from './fixtures';

// Render smoke tests for the kit primitives — they mount with a fixture and show
// the expected content + carry the variant's token class, so a screen built on
// them renders (rather than crashing) without a live backend.
describe('kit primitives render', () => {
	afterEach(cleanup);

	it('Icon renders the bare name class + tone', () => {
		// The DOM class is the bare Solar name (`command`); the `i-solar:*-linear`
		// expansion lives in generated CSS via rokkit.config `icons.overrides`.
		const { container } = render(Icon, { name: 'command', toneClass: 'text-accent' });
		const el = container.firstElementChild as HTMLElement;
		expect(el.className).toContain('command');
		expect(el.className).toContain('text-accent');
	});

	it('KanjiToken paints the glyph at the named size', () => {
		const { getByText } = render(KanjiToken, { char: '結', size: 'xl' });
		const el = getByText('結');
		expect(el.getAttribute('style')).toContain('font-size: 22px');
	});

	it('ClassChip shows the classification label + tone', () => {
		const { getByText } = render(ClassChip, { kind: 'client' });
		const el = getByText('client');
		expect(el.className).toContain('text-accent');
	});

	it('RoleTag shows the role label, and renders nothing for an unknown role', () => {
		const { getByText } = render(ClassChip, { kind: 'company' });
		expect(getByText('company')).toBeTruthy();
		const { getByText: g2 } = render(RoleTag, { role: 'maintainer' });
		expect(g2('maintainer')).toBeTruthy();
		const { container } = render(RoleTag, { role: 'nobody' });
		expect(container.textContent?.trim()).toBe('');
	});

	it('PhasePill shows the phase label', () => {
		const { getByText } = render(PhasePill, { phase: 'adopt' });
		const el = getByText('adopt');
		expect(el.className).toContain('text-success');
	});

	it('StatBadge shows the number, label and sub', () => {
		const { getByText } = render(StatBadge, { n: 612, label: 'helped', sub: 'this month' });
		expect(getByText('612')).toBeTruthy();
		expect(getByText('helped')).toBeTruthy();
		expect(getByText('this month')).toBeTruthy();
	});

	it('ProjectRow shows name, repo, classification and needs count', () => {
		const { getByText } = render(ProjectRow, { p: projects[0] });
		expect(getByText('lumen-auth')).toBeTruthy();
		expect(getByText('acme/lumen-auth')).toBeTruthy();
		expect(getByText('company')).toBeTruthy();
		expect(getByText('notice')).toBeTruthy();
	});

	it('ProjectRow is one row at every width — phone cells always, table cells at md', () => {
		// Replaces a `compact` prop that rendered a second, near-identical block.
		// The phone cells (name · repo · needs · phase · lastRun) are unconditional;
		// the four table-only cells are in the DOM but hidden below md, so CSS — not
		// a prop — picks the layout.
		const { getByText, container } = render(ProjectRow, { p: projects[0] });
		expect(getByText('lumen-auth')).toBeTruthy();
		expect(getByText('8m')).toBeTruthy();
		const row = container.querySelector('button')!;
		expect(row.className).toContain('flex');
		expect(row.className).toContain('md:grid');
		// classification is a table-only cell: present, but phone-hidden.
		const cls = getByText('company').closest('span[class*="hidden"]');
		expect(cls).toBeTruthy();
		expect(cls!.className).toContain('md:flex');
	});

	it('MyDojoRow shows the dōjō identity, meta and needs chip', () => {
		const { getByText } = render(MyDojoRow, { dojo: dojos[0] });
		expect(getByText('Acme Corp')).toBeTruthy();
		expect(getByText(/48 members/)).toBeTruthy();
		expect(getByText('4 need you')).toBeTruthy();
	});

	it('MyDojoRow omits the needs chip when nothing needs you', () => {
		const { queryByText } = render(MyDojoRow, { dojo: dojos[2] });
		expect(queryByText(/need you/)).toBeNull();
	});

	it('ConfidenceBar shows the rounded percent', () => {
		const { getByText } = render(ConfidenceBar, { v: 0.91 });
		expect(getByText('91')).toBeTruthy();
	});

	it('Spark and Enso mount (reused shipped primitives)', () => {
		const { container } = render(Spark, { data: [1, 2, 3, 4] });
		expect(container.querySelector('svg')).toBeTruthy();
		const { getByText } = render(Enso, { progress: 0.5, label: '50' });
		expect(getByText('50')).toBeTruthy();
	});
});
