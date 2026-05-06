<script lang="ts">
  let { onadd, placeholder = '~/Developer', scanning = false }: {
    onadd: (path: string) => void;
    placeholder?: string;
    scanning?: boolean;
  } = $props();

  let input = $state('');

  function add() {
    const path = input.trim();
    if (path) { onadd(path); input = ''; }
  }

  async function browse() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const picked = await invoke<string | null>('pick_folder');
      if (picked) onadd(picked);
    } catch { /* browser preview — no Tauri */ }
  }
</script>

<div class="flex gap-2">
  <input
    type="text"
    bind:value={input}
    onkeydown={(e) => { if (e.key === 'Enter') add(); }}
    {placeholder}
    class="min-w-0 flex-1 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4"
  />
  <button onclick={add} disabled={!input.trim() || scanning}
    class="rounded-lg bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3 disabled:opacity-50">
    {scanning ? 'Scanning…' : 'Add'}
  </button>
  <button onclick={browse} title="Browse"
    class="rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 text-surface-z5 hover:bg-surface-z3 transition-colors">
    <span class="i-solar-folder-open-bold-duotone text-sm"></span>
  </button>
</div>
