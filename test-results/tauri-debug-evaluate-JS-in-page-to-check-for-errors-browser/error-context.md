# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: tauri-debug.spec.ts >> evaluate JS in page to check for errors
- Location: e2e/tests/tauri-debug.spec.ts:45:1

# Error details

```
Error: expect(received).toBeGreaterThan(expected)

Expected: > 0
Received:   0
```

# Test source

```ts
  1  | /**
  2  |  * Debug test — connects to the running Tauri app and captures console errors.
  3  |  * Run with: cargo tauri dev --features e2e-testing
  4  |  * Then:     npx playwright test --config e2e/playwright.config.ts e2e/tests/tauri-debug.spec.ts
  5  |  */
  6  | 
  7  | import { test, expect } from '../fixtures';
  8  | 
  9  | test('capture console errors on health page', async ({ tauriPage }) => {
  10 |   const errors: string[] = [];
  11 | 
  12 |   // Listen for console errors if the API supports it
  13 |   try {
  14 |     const page = (tauriPage as any)._page || tauriPage;
  15 |     if (page.on) {
  16 |       page.on('console', (msg: any) => {
  17 |         if (msg.type() === 'error') {
  18 |           errors.push(msg.text());
  19 |         }
  20 |       });
  21 |       page.on('pageerror', (err: any) => {
  22 |         errors.push(err.message || String(err));
  23 |       });
  24 |     }
  25 |   } catch { /* tauriPage might not expose page events */ }
  26 | 
  27 |   await tauriPage.goto('/health');
  28 | 
  29 |   // Wait a moment for any async errors
  30 |   await new Promise(r => setTimeout(r, 2000));
  31 | 
  32 |   // Navigate to setup
  33 |   await tauriPage.goto('/setup/welcome');
  34 |   await new Promise(r => setTimeout(r, 1000));
  35 | 
  36 |   // Report any errors captured
  37 |   if (errors.length > 0) {
  38 |     console.log('Console errors captured:', errors);
  39 |   }
  40 | 
  41 |   // The test passes but logs errors for debugging
  42 |   expect(true).toBe(true);
  43 | });
  44 | 
  45 | test('evaluate JS in page to check for errors', async ({ tauriPage }) => {
  46 |   await tauriPage.goto('/setup/welcome');
  47 | 
  48 |   // Try to evaluate JS to check the page state
  49 |   try {
  50 |     const result = await (tauriPage as any).evaluate('document.title');
  51 |     console.log('Page title:', result);
  52 |   } catch (e) {
  53 |     console.log('evaluate not available:', e);
  54 |   }
  55 | 
  56 |   // Check if the hero element exists
  57 |   const hero = tauriPage.locator('.hero, .wiz-header');
  58 |   const count = await hero.count();
  59 |   console.log('Hero/header elements found:', count);
> 60 |   expect(count).toBeGreaterThan(0);
     |                 ^ Error: expect(received).toBeGreaterThan(expected)
  61 | });
  62 | 
```