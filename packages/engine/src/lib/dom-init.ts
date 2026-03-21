// packages/engine/src/lib/dom-init.ts
// Must be imported before @mozilla/readability — it checks global.DOMParser at module init
// time and falls back to require("@mixmark-io/domino") (CJS) when it's undefined. Providing
// linkedom's DOMParser here prevents that CJS require from running in ESM SSR bundles.
import { DOMParser } from "linkedom";

// @mozilla/readability resolves its parser as: root = typeof window !== "undefined" ? window : {}
// In Node.js, window is undefined so root = {} and root.DOMParser is undefined, triggering a
// require("@mixmark-io/domino") CJS fallback that breaks ESM SSR bundles.
// Setting globalThis.window = globalThis makes root = global so root.DOMParser picks up ours.
const g = globalThis as Record<string, unknown>;
if (typeof g["window"] === "undefined") g["window"] = globalThis;
if (typeof g["DOMParser"] === "undefined") g["DOMParser"] = DOMParser;
