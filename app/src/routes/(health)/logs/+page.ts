import { invoke } from '@tauri-apps/api/core';
import { hasTauri } from '$lib/bootstrap.js';
import type { PageLoad } from './$types';
import type { LogSession } from '$lib/types.js';

export const load: PageLoad = async () => {
    if (!hasTauri()) return { sessions: [] };
    const sessions = await invoke<LogSession[]>('get_log_sessions', { module: null });
    return { sessions };
};
