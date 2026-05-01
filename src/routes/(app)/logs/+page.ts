// app/src/routes/(app)/logs/+page.ts
import { invoke } from '@tauri-apps/api/core';
import type { PageLoad } from './$types';
import type { LogSession } from '$lib/types.js';

export const load: PageLoad = async () => {
    const sessions = await invoke<LogSession[]>('get_log_sessions', { module: null });
    return { sessions };
};
