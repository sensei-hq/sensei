<script lang="ts">
	import 'uno.css';
	import '../app.css';
	import { vibe } from '@rokkit/states';
	import { themable } from '@rokkit/actions';
	import { setContext, onMount } from 'svelte';
	import { page } from '$app/stores';

	let { children } = $props();

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
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-console-theme' }} />

{@render children()}
