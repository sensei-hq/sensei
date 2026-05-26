import { defineConfig, transformerDirectives } from "unocss";
import { presetRokkit } from "@rokkit/unocss";
import config from "./rokkit.config.js";

/**
 * Source of truth: docs/mockups/lib/tokens.css.
 * Every value below traces back to a stop in the mockup spec.
 */
export default defineConfig({
  transformers: [transformerDirectives()],
  presets: [presetRokkit(config)],
  theme: {
    fontSize: {
      xs:    ["11px", { lineHeight: "1.4" }],
      sm:    ["13px", { lineHeight: "1.5" }],
      base:  ["15px", { lineHeight: "1.6" }],
      lg:    ["17px", { lineHeight: "1.5" }],
      xl:    ["22px", { lineHeight: "1.2" }],
      "2xl": ["28px", { lineHeight: "1.2" }],
      "3xl": ["40px", { lineHeight: "1.2" }],
      "4xl": ["56px", { lineHeight: "1.05" }],
    },
    letterSpacing: {
      tight:  "-0.02em",
      normal: "0",
      wide:   "0.18em",
    },
    lineHeight: {
      tight:  "1.2",
      snug:   "1.4",
      normal: "1.6",
      loose:  "1.75",
    },
    borderRadius: {
      sm:      "4px",
      DEFAULT: "6px",
      lg:      "10px",
      full:    "9999px",
    },
    transitionDuration: {
      fast:    "120ms",
      DEFAULT: "180ms",
      slow:    "280ms",
    },
    transitionTimingFunction: {
      DEFAULT: "cubic-bezier(0.2, 0.6, 0.2, 1)",
    },
    boxShadow: {
      sm:      "0 1px 2px oklch(var(--color-ink-z9) / 0.04)",
      DEFAULT: "0 1px 3px oklch(var(--color-ink-z9) / 0.06), 0 8px 24px oklch(var(--color-ink-z9) / 0.06)",
      lg:      "0 24px 60px oklch(var(--color-ink-z9) / 0.18)",
    },
  },
});
