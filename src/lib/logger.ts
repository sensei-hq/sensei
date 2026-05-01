/**
 * Module-scoped session logger.
 *
 * Each module (bootstrap, wizard, projects…) gets one active session at a time.
 * Log entries are forwarded to the Tauri sidecar via invoke — the sidecar is the
 * sole writer to disk.
 *
 * Usage:
 *   const logger = getModuleLogger('wizard');
 *   await logger.ready;
 *   logger.info('ui', 'prefs_save', 'Preferences saved', { ms: 58 });
 *   // In onDestroy / beforeNavigate:
 *   logger.close();
 */

import { invoke } from '@tauri-apps/api/core';
import type { LogEntry } from './types.js';

// ── Entry ID helper ────────────────────────────────────────────────────────

let _counter = 0;
function nextEntryId(): string {
    return `entry-${Date.now()}-${++_counter}`;
}

function nowIso(): string {
    return new Date().toISOString();
}

// ── ModuleLogger ───────────────────────────────────────────────────────────

export class ModuleLogger {
    readonly module: string;
    /** Resolves when the session has been started with the sidecar. */
    readonly ready: Promise<void>;

    private _sessionId = '';
    private _startPromise: Promise<void>;

    constructor(module: string) {
        this.module = module;
        this._startPromise = this._startSession();
        this.ready = this._startPromise;
    }

    private async _startSession(): Promise<void> {
        const appVersion = '0.1.0';
        const systemInfo = {
            os: navigator.userAgent,
            arch: 'unknown',
            ram_gb: 0,
            cpu_cores: navigator.hardwareConcurrency ?? 0,
        };
        this._sessionId = await invoke<string>('log_session_start', {
            module:      this.module,
            app_version: appVersion,
            system_info: systemInfo,
        });
    }

    async info(
        layer: LogEntry['layer'],
        step: string,
        msg: string,
        data?: Record<string, unknown>,
    ): Promise<void> {
        await this._send({ level: 'info', layer, step, msg, data });
    }

    async warn(
        layer: LogEntry['layer'],
        step: string,
        msg: string,
        data?: Record<string, unknown>,
    ): Promise<void> {
        await this._send({ level: 'warn', layer, step, msg, data });
    }

    async error(
        layer: LogEntry['layer'],
        step: string,
        msg: string,
        errOrData?: Error | Record<string, unknown>,
    ): Promise<void> {
        const extra: Partial<LogEntry> = {};
        if (errOrData instanceof Error) {
            extra.err   = errOrData.message;
            extra.stack = errOrData.stack;
        } else if (errOrData) {
            extra.data = errOrData;
        }
        await this._send({ level: 'error', layer, step, msg, ...extra });
    }

    async close(): Promise<void> {
        await this._startPromise;
        await invoke('log_session_end', { session_id: this._sessionId });
        _registry.delete(this.module);
    }

    private async _send(fields: Partial<LogEntry>): Promise<void> {
        await this._startPromise;
        const entry: LogEntry = {
            id:    nextEntryId(),
            ts:    nowIso(),
            level: fields.level ?? 'info',
            layer: fields.layer ?? 'ui',
            step:  fields.step  ?? '',
            msg:   fields.msg   ?? '',
            data:  fields.data,
            err:   fields.err,
            stack: fields.stack,
        };

        if (import.meta.env.DEV) {
            const tag = { module: this.module, layer: entry.layer };
            if (entry.level === 'error') console.error('[sensei:logger]', tag, entry.msg, entry);
            else if (entry.level === 'warn') console.warn('[sensei:logger]', tag, entry.msg, entry);
            else console.debug('[sensei:logger]', tag, entry.msg, entry);
        }

        await invoke('log_entry', {
            session_id: this._sessionId,
            entry,
        });
    }
}

// ── Module registry ────────────────────────────────────────────────────────

const _registry = new Map<string, ModuleLogger>();

/**
 * Return (or create) the ModuleLogger for the given module.
 * Calling this twice with the same module returns the same instance.
 */
export function getModuleLogger(module: string): ModuleLogger {
    let logger = _registry.get(module);
    if (!logger) {
        logger = new ModuleLogger(module);
        _registry.set(module, logger);
    }
    return logger;
}
