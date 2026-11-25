import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// The pre-paint script in app.html decides the title bar before first paint,
// so it cannot import a module: this runs the script itself against a fake
// document, the way the webview would.
const appHtml = readFileSync(fileURLToPath(new URL('../app.html', import.meta.url)), 'utf8');
const appCss = readFileSync(fileURLToPath(new URL('../app.css', import.meta.url)), 'utf8');

function titlebarFor(userAgent: string): string | undefined {
    const script = appHtml.match(/<script>([\s\S]*?)<\/script>/)?.[1];
    if (!script) throw new Error('app.html has no pre-paint script');
    const html = { dataset: {} as Record<string, string> };
    const body = { dataset: {} as Record<string, string> };
    const run = new Function('document', 'navigator', 'window', 'localStorage', script);
    run(
        { documentElement: html, body },
        { userAgent },
        { matchMedia: () => ({ matches: false }) },
        { getItem: () => null },
    );
    return html.dataset.titlebar;
}

describe('title bar before first paint', () => {
    it('is the overlay on macOS', () => {
        expect(
            titlebarFor('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)'),
        ).toBe('overlay');
    });

    it('is the native one on Windows (WebView2)', () => {
        expect(
            titlebarFor(
                'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0',
            ),
        ).toBe('native');
    });

    it('is the native one on Linux (WebKitGTK)', () => {
        expect(titlebarFor('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)')).toBe('native');
    });

    it('drops the empty drag strips and narrows the inset only for a native title bar', () => {
        expect(appCss).toMatch(/html\[data-titlebar="native"\] \.drag-region:empty \{\s*display: none;/);
        expect(appCss).toMatch(/\.titlebar-inset \{\s*padding-left: 80px;/);
        expect(appCss).toMatch(/html\[data-titlebar="native"\] \.titlebar-inset \{\s*padding-left: 1rem;/);
    });
});
