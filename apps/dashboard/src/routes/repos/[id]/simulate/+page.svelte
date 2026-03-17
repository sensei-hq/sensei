<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageData, ActionData } from './$types';

  const { data, form }: { data: PageData; form: ActionData } = $props();

  let query = $state('');
  let loading = $state(false);
  let expandedIdx = $state<number | null>(null);

  function pct(similarity: number | undefined): string {
    if (similarity == null) return '';
    return `${Math.round(similarity * 100)}%`;
  }

  function matchColor(similarity: number | undefined): string {
    if (similarity == null) return 'bg-surface-z2 text-surface-z5 border-surface-z3';
    if (similarity >= 0.80) return 'bg-success-z1 text-success-z7 border-success-z3';
    if (similarity >= 0.65) return 'bg-warning-z1 text-warning-z7 border-warning-z3';
    return 'bg-surface-z2 text-surface-z5 border-surface-z3';
  }
</script>

<div class="mb-6">
  <a href="/repos/{data.repo.id}" class="text-sm text-surface-z5 hover:text-surface-z7 transition-colors">
    ← {data.repo.name}
  </a>
</div>

<h1 class="text-2xl font-semibold text-surface-z8 mb-1">Simulate Library Query</h1>
<p class="text-sm text-surface-z5 mb-2">
  Mirrors what Claude receives from <code class="font-mono text-xs bg-surface-z2 px-1 rounded">get_lib_docs</code>.
  Searches all {data.libs.length} linked lib{data.libs.length !== 1 ? 's' : ''} in parallel:
  <span class="text-surface-z6">{data.libs.join(', ')}</span>
</p>

<form
  method="POST"
  action="?/simulate"
  use:enhance={() => {
    loading = true;
    expandedIdx = null;
    return async ({ update }) => { await update(); loading = false; };
  }}
  class="flex gap-3 mb-8 flex-wrap"
>
  <input
    name="query"
    type="text"
    bind:value={query}
    placeholder="e.g. how do I configure kavach"
    class="flex-1 min-w-72 px-3 py-2 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z8 placeholder-surface-z4 focus:outline-none focus:border-primary-z5"
  />
  <button
    type="submit"
    disabled={loading || !query.trim()}
    class="px-4 py-2 rounded-lg bg-primary-z5 text-on-primary text-sm font-medium disabled:opacity-40 hover:bg-primary-z6 transition-colors"
  >
    {loading ? 'Searching…' : 'Search'}
  </button>

  {#if form?.error}
    <p class="w-full text-sm text-error-z6">{form.error}</p>
  {/if}
</form>

{#if form?.sections}
  <div class="mb-3">
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">
      {form.sections.length} section{form.sections.length !== 1 ? 's' : ''} matched
      <span class="text-surface-z4 normal-case font-normal">· "{form.query}"</span>
    </h2>
  </div>

  {#if form.sections.length === 0}
    <p class="text-sm text-surface-z5">No sections matched. Try re-indexing or building embeddings for the linked libraries.</p>
  {:else}
    <div class="flex flex-col gap-2">
      {#each form.sections as section, i}
        {@const expanded = expandedIdx === i}
        <div class="rounded-lg border border-surface-z3 bg-surface-z1 overflow-hidden">
          <button
            type="button"
            onclick={() => expandedIdx = expanded ? null : i}
            class="w-full text-left px-4 py-3 flex items-start gap-3 hover:bg-surface-z2 transition-colors"
          >
            <!-- Match % badge -->
            {#if section.similarity != null}
              <span class="mt-0.5 shrink-0 text-xs font-mono font-semibold px-1.5 py-0.5 rounded border {matchColor(section.similarity)}">
                {pct(section.similarity)}
              </span>
            {/if}

            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 flex-wrap mb-0.5">
                <!-- Library badge -->
                <span class="text-xs font-semibold px-1.5 py-0.5 rounded bg-primary-z1 text-primary-z6 border border-primary-z3">
                  {section.lib}
                </span>
                <span class="text-sm font-medium text-surface-z8 truncate">{section.title}</span>
              </div>
              <div class="text-xs text-surface-z5 flex items-center gap-2 flex-wrap">
                <span>{section.document.title}</span>
                {#if section.document.component}
                  <span class="px-1.5 py-0.5 rounded bg-surface-z2 border border-surface-z3">{section.document.component}</span>
                {/if}
                {#if section.document.url}
                  <a
                    href={section.document.url}
                    target="_blank"
                    rel="noopener"
                    onclick={e => e.stopPropagation()}
                    class="text-primary-z5 hover:underline"
                  >↗</a>
                {/if}
              </div>
            </div>

            <span class="text-surface-z4 text-xs mt-0.5 shrink-0">{expanded ? '▲' : '▼'}</span>
          </button>

          {#if expanded}
            <div class="border-t border-surface-z3 px-4 py-3 bg-surface-z0">
              <p class="text-xs text-surface-z5 mb-2 font-mono uppercase tracking-wider">
                Raw content sent to Claude · <span class="text-primary-z5">{section.lib}</span>
              </p>
              <pre class="text-xs text-surface-z7 font-mono whitespace-pre-wrap overflow-x-auto leading-relaxed">{section.content}</pre>
              {#if section.document.summary}
                <p class="text-xs text-surface-z5 mt-3 border-t border-surface-z3 pt-2">
                  <span class="font-medium">Doc summary:</span> {section.document.summary}
                </p>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
{/if}
