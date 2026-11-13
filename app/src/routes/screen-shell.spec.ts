import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

// #293: every observatory and workspace screen lays out through a shared layout
// — one header that stays in place over one scroll body — instead of
// improvising its own header and scroll. `Screen` (@sensei/ui) is that layout
// today; D87 moves each screen onto rokkit's `ScreenShell` as its lane touches
// it, so a screen may render either, and neither one may be hand-rolled. The
// window layouts then own no scroll of their own.
const ROOT = 'src/routes';
const GROUPS = ['(observatory)', '(workspace)'];

function pages(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return pages(path);
		return name === '+page.svelte' ? [relative(ROOT, path)] : [];
	});
}

const source = (page: string) => readFileSync(join(ROOT, page), 'utf8');
// The markup outside a component's <script> blocks: every block, any case,
// repeated until nothing is left to strip.
const withoutScripts = (text: string) => {
	let out = text;
	for (let prev = ''; prev !== out; ) {
		prev = out;
		out = out.replace(/<script\b[\s\S]*?<\/script[^>]*>/gi, '');
	}
	return out;
};
// A page with no markup only redirects (its `+page.ts` throws); it is not a screen.
const hasMarkup = (page: string) => withoutScripts(source(page)).trim() !== '';
const all = GROUPS.flatMap((g) => pages(join(ROOT, g))).filter(hasMarkup).sort();
// A page that renders a `…Screen.svelte` — co-located, or shared from
// `$lib/components/<dir>` — lays out through whatever that component renders,
// so the component is read too.
const usesSharedLayout = (page: string) => {
	const own = source(page);
	const local = own.match(/import \w+ from '\.\/(\w+Screen\.svelte)'/);
	const shared = own.match(/import \{[^}]*\b(\w+Screen)\b[^}]*\} from '\$lib\/components\/(\w+)'/);
	// A screen shared from a feature directory (`$lib/relay/RelayScreen.svelte`), for a
	// screen two windows render.
	const feature = own.match(/import \w+ from '\$lib\/([\w-]+)\/(\w+\.svelte)'/);
	const delegate = local
		? source(join(dirname(page), local[1]))
		: shared
			? readFileSync(join('src/lib/components', shared[2], `${shared[1]}.svelte`), 'utf8')
			: feature
				? readFileSync(join('src/lib', feature[1], feature[2]), 'utf8') + featureShell(feature[1], feature[2])
				: '';
	const text = own + delegate;
	return /<Screen[\s>]/.test(text) || rendersRokkitShell(text) || rendersSharedShell(text);
};
// A feature screen may draw its shell through a sibling `…Shell.svelte`
// (`RelayScreen` → `RelayShell` → `ShellScreen`); that sibling is read too.
const featureShell = (dir: string, file: string) => {
	const shell = readFileSync(join('src/lib', dir, file), 'utf8').match(/import \w+ from '\.\/(\w+Shell\.svelte)'/);
	return shell ? readFileSync(join('src/lib', dir, shell[1]), 'utf8') : '';
};
// D87: a migrated screen renders rokkit's `ScreenShell` — imported from
// `@rokkit/app`, not a lookalike carrying the same name.
const rendersRokkitShell = (text: string) =>
	/<ScreenShell[\s>]/.test(text) &&
	/import\s*\{[^}]*\bScreenShell\b[^}]*\}\s*from\s*['"]@rokkit\/app['"]/.test(text);
// D87: `@sensei/ui`'s ShellScreen is rokkit's shell with the host's glyph mode and
// panel store wired in — the shared layout a screen renders instead of rokkit's own.
const rendersSharedShell = (text: string) =>
	/<ShellScreen[\s>]/.test(text) &&
	/import\s*\{[^}]*\bShellScreen\b[^}]*\}\s*from\s*['"]@sensei\/ui['"]/.test(text);

describe('screens use a shared screen layout (#293, D87)', () => {
	it('every screen uses Screen, or rokkit’s ScreenShell', () => {
		expect(all.filter((p) => !usesSharedLayout(p))).toEqual([]);
	});

	it('the window layouts do not scroll — each screen body is the one scroller', () => {
		const windows = ['(observatory)/+layout.svelte', '(workspace)/workspace/[id]/+layout.svelte'];
		const scrolling = windows.filter((layout) => {
			const markup = withoutScripts(source(layout));
			const main = markup.match(/<main\b[^>]*>/)?.[0];
			expect(main, layout).toBeDefined();
			return /overflow-(auto|scroll|y-auto|y-scroll)\b/.test(main!);
		});
		expect(scrolling).toEqual([]);
	});
});
