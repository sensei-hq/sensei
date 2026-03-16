<!-- apps/dashboard/src/lib/components/AddLibrarySidebar.svelte -->
<script lang="ts">
  import { enhance } from '$app/forms';

  interface Props {
    open: boolean;
    action: string;
    onclose: () => void;
  }

  const { open, action, onclose }: Props = $props();
</script>

{#if open}
  <div
    class="fixed inset-0 bg-black/40 z-40"
    role="presentation"
    onclick={onclose}
  ></div>

  <div class="fixed right-0 top-0 h-full w-96 bg-surface-z1 border-l border-surface-z3 z-50 flex flex-col shadow-xl">
    <div class="flex items-center justify-between px-5 py-4 border-b border-surface-z3 sticky top-0 bg-surface-z1">
      <h2 class="text-sm font-semibold text-surface-z8">Add Library</h2>
      <button onclick={onclose} class="text-surface-z5 hover:text-surface-z8 transition-colors" aria-label="Close">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18">
          <path d="M18 6L6 18M6 6l12 12" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
    <form method="POST" {action} use:enhance class="flex flex-col gap-4 px-5 py-5 flex-1">
      <label class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-surface-z6">Name <span class="text-error-z6">*</span></span>
        <input
          type="text"
          name="name"
          required
          placeholder="e.g. rokkit"
          class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none"
        />
      </label>
      <label class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-surface-z6">Doc URL <span class="text-error-z6">*</span></span>
        <input
          type="text"
          name="url"
          required
          placeholder="https://docs.example.com/llms.txt or https://github.com/org/repo/tree/main/docs"
          class="px-3 py-2 rounded border border-surface-z3 bg-surface-z2 text-surface-z8 text-sm focus:border-primary-z5 focus:outline-none"
        />
        <span class="text-xs text-surface-z4">
          Supports llms.txt URLs, GitHub folder URLs, and HTTP doc pages.
          Local folders must be indexed via CLI: <code>sensei index</code>
        </span>
      </label>
      <div class="flex gap-2 mt-auto">
        <button
          type="submit"
          class="flex-1 px-4 py-2 rounded bg-primary-z6 text-white text-sm font-medium hover:bg-primary-z7 transition-colors"
        >
          Add & Index
        </button>
        <button
          type="button"
          onclick={onclose}
          class="px-4 py-2 rounded border border-surface-z3 bg-surface-z1 text-surface-z6 text-sm hover:border-primary-z5 transition-colors"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
{/if}
