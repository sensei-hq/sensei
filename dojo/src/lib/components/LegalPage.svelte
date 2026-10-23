<script lang="ts">
	// A Terms of Service or Privacy Policy page (#330): presentational; the words
	// and facts live in $lib/legal-content.
	import { LEGAL_PAGES, type LegalPage } from '$lib/legal-content';

	let { page }: { page: LegalPage } = $props();

	const others = $derived(LEGAL_PAGES.filter((p) => p.path !== page.path));
</script>

<svelte:head>
	<title>{page.title} · Dōjō</title>
</svelte:head>

<main class="bg-paper text-ink min-h-screen px-6 py-12">
	<article class="mx-auto max-w-[680px]">
		<p class="text-ink-mute m-0 text-xs font-semibold uppercase" style="letter-spacing: 0.2em">Dōjō</p>
		<h1 class="display m-0 mt-2 font-light text-3xl">{page.title}</h1>
		<p class="text-ink-mute mt-2 text-sm">Last updated {page.updated}</p>

		{#each page.sections as section (section.heading)}
			<section class="mt-8">
				<h2 class="m-0 font-heading text-lg">{section.heading}</h2>
				{#each section.body as para, i (i)}
					<p class="text-ink-soft mt-3 text-sm" style="line-height: 1.6">{para}</p>
				{/each}
			</section>
		{/each}

		<nav class="border-paper-edge mt-12 flex flex-wrap gap-4 border-t pt-4 text-sm" aria-label="Legal">
			{#each others as other (other.path)}
				<a href={other.path} class="text-primary underline">{other.title}</a>
			{/each}
			<a href="/signin" class="text-primary underline">Sign in</a>
		</nav>
	</article>
</main>
