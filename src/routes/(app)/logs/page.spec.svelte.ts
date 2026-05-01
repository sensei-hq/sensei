// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import Page from './+page.svelte';
import type { LogSession, BootstrapTrace } from '$lib/types.js';

// Mock Tauri opener — not available in test env
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));

afterEach(() => cleanup());

function makeTrace(step: string, ok: boolean): BootstrapTrace {
    return {
        id: step, ts: '2026-05-01T10:00:00Z', action_type: 'check',
        step, desc: step, cmd: `which ${step}`, exit: ok ? 0 : 1,
        out: ok ? '/usr/bin/' + step : '', err: ok ? '' : 'not found',
        ms: 12, ok, fix_attempted: false, fix_approach: null, fix_ok: null,
    };
}

const bootstrapSession: LogSession = {
    id: 'sess-00000001',
    module: 'bootstrap',
    started_at: new Date().toISOString(),
    app_version: '0.1.0',
    system_info: { os: 'macOS 15.0', arch: 'arm64', ram_gb: 16, cpu_cores: 10 },
    outcome: 'success',
    duration_ms: 1234,
    traces: [makeTrace('postgres', true), makeTrace('ollama', false)],
};

const wizardSession: LogSession = {
    id: 'sess-00000002',
    module: 'wizard',
    started_at: new Date().toISOString(),
    app_version: '0.1.0',
    system_info: { os: 'macOS 15.0', arch: 'arm64', ram_gb: 16, cpu_cores: 10 },
    outcome: 'partial',
    duration_ms: 456,
    traces: [],
};

describe('/logs page', () => {
    it('renders session sidebar with date group and module sections', () => {
        const { getAllByText, getByText } = render(Page, {
            props: { data: { sessions: [bootstrapSession, wizardSession] } },
        });
        expect(getByText('Today')).toBeTruthy();
        // "Bootstrap" appears in both module-label (BOOTSTRAP) and trace-title (Bootstrap)
        expect(getAllByText(/Bootstrap/i).length).toBeGreaterThan(0);
        expect(getAllByText(/Setup Wizard|Wizard/i).length).toBeGreaterThan(0);
    });

    it('renders trace table when session is selected', async () => {
        const { getAllByText } = render(Page, {
            props: { data: { sessions: [bootstrapSession] } },
        });
        // First session is auto-selected — trace step names appear in table
        expect(getAllByText('postgres').length).toBeGreaterThan(0);
    });

    it('opens report modal when "Report this session" is clicked', async () => {
        const { getByText } = render(Page, {
            props: { data: { sessions: [bootstrapSession] } },
        });
        const reportBtn = getByText(/Report this session/i);
        await reportBtn.click();
        expect(getByText(/ISSUE PREVIEW/i)).toBeTruthy();
    });

    it('anonymizes paths in issue body', async () => {
        const sessionWithPath: LogSession = {
            ...bootstrapSession,
            traces: [{
                ...makeTrace('pg', true),
                cmd: 'which /Users/jerry/homebrew/bin/postgres',
                out: '/Users/jerry/homebrew/bin/postgres',
            }],
        };
        const { getByText, container } = render(Page, {
            props: { data: { sessions: [sessionWithPath] } },
        });
        const reportBtn = getByText(/Report this session/i);
        await reportBtn.click();
        const preview = container.querySelector('.body-preview');
        expect(preview?.textContent).not.toContain('/Users/jerry/');
        expect(preview?.textContent).toContain('~/');
    });
});
