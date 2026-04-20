<script lang="ts">
  import { STATUS_CLS, INDEX_STATUS_CLS, ROLE_CLS, CAT_CLS } from './colors.js';

  let { status, variant = 'status' }: {
    status: string;
    variant?: 'status' | 'index' | 'role' | 'category';
  } = $props();

  const maps: Record<string, Record<string, string>> = {
    status: STATUS_CLS, index: INDEX_STATUS_CLS, role: ROLE_CLS, category: CAT_CLS,
  };

  const cls = $derived(maps[variant]?.[status] ?? 'bg-surface-z3 text-surface-z5');
</script>

<span class="rounded px-1.5 py-0.5 text-[10px] font-medium {cls}">{status}</span>
