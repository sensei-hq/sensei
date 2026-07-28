<script lang="ts">
	import 'uno.css';
	import '../app.css';
	import { vibe } from '@rokkit/states';
	import { themable } from '@rokkit/actions';
	import { setContext, onMount } from 'svelte';
	import { page } from '$app/stores';
	import { colorMode, loadColorMode, setColorMode } from '$lib/theme.svelte';

	let { children } = $props();

	// The console ports the desktop app's Zen/Sumi vocabulary (see rokkit.config.js
	// + app.css), so the effective theme must be `zen-sumi`, not rokkit. The shared
	// `vibe` singleton defaults to style `rokkit` and its allowed styles are
	// ['rokkit','minimal','material'] — its `style` setter silently rejects any
	// value not in `allowedStyles`. The `themable` action's effect unconditionally
	// writes `vibe.style` onto <body>/<html>, so without this it would overwrite the
	// `data-style="zen-sumi"` set in app.html back to rokkit. Register zen-sumi as
	// an allowed style and select it before `themable` attaches on mount. The app is
	// light-mode only, so pin mode/density to match app.html too.
	vibe.allowedStyles = ['zen-sumi'];
	vibe.style = 'zen-sumi';
	// `light` is only the pre-hydration default (no-JS / SSR); on mount the persisted
	// or system-derived color mode is applied + kept in sync with the OS (see below).
	vibe.mode = 'light';
	vibe.density = 'comfortable';

	// Mirror sites/demo/src/routes/(app)/+layout@.svelte: hydrate a browser-side
	// kavach instance from the generated $kavach/auth module and expose it via
	// context so descendant auth components (e.g. @kavach/ui AuthProvider) can
	// call signIn/onAuthChange. Guarded to onMount so SSR/prerender never touches
	// the supabase browser client.
	// createKavach is typed as `object` upstream; narrow to the members this app
	// calls so the assignment + onAuthChange are type-checked.
	type KavachInstance = Record<string, unknown> & {
		onAuthChange: (url: URL) => void;
	};

	const kavach = $state<Record<string, unknown>>({});
	setContext('kavach', kavach);

	// Apply the persisted / system color mode client-side + follow OS changes.
	onMount(() => {
		setColorMode(loadColorMode());
		return colorMode.listen();
	});

	onMount(async () => {
		const { createKavach } = await import('kavach');
		const { adapter, logger } = await import('$kavach/auth');
		const { invalidateAll } = await import('$app/navigation');
		const instance = createKavach(adapter, { logger, invalidateAll }) as KavachInstance;
		Object.assign(kavach, instance);
		instance.onAuthChange($page.url);
	});
</script>

<svelte:head>
	<title>Dōjō · sensei</title>
	<meta name="description" content="Dōjō — your organization's shared mind" />
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-dojo-theme' }} />

{@render children()}
