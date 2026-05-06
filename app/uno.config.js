import { defineConfig, transformerDirectives } from "unocss";
import { presetRokkit } from "@rokkit/unocss";
import config from "./rokkit.config.js";

export default defineConfig({
  transformers: [transformerDirectives()],
  presets: [presetRokkit(config)],
  theme: {
    // Custom font sizes for UI token scale
    // Standard Tailwind: xs=12, sm=14, base=16, lg=18, xl=20, 2xl=24, 3xl=30, 4xl=36, 5xl=48, 6xl=60, 7xl=72
    fontSize: {
      nano:  ["9px",   { lineHeight: "1.4" }], // tiny meta labels
      micro: ["9.5px", { lineHeight: "1.4" }], // sidebar section headers
      "3xs": ["10px",  { lineHeight: "1.4" }], // very small labels
      "2xs": ["11px",  { lineHeight: "1.4" }], // small labels, badges
      ui:    ["13px",  { lineHeight: "1.5" }], // standard UI text (default)
      body:  ["15px",  { lineHeight: "1.5" }], // comfortable body text
      prose: ["17px",  { lineHeight: "1.5" }], // roomy reading text
      wm:    ["220px", { lineHeight: "1"   }], // watermark / decorative kanji
    },
    // Custom letter-spacing tokens (complements Tailwind's tighter/tight/normal/wide/wider/widest)
    letterSpacing: {
      cap:   "0.12em", // all-caps short labels
      tag:   "0.14em", // medium-spaced tags
      label: "0.16em", // UI section labels
      loose: "0.18em", // spacious display text
    },
    // Custom line-height token
    lineHeight: {
      reading: "1.7", // comfortable reading line height
    },
    // Off-grid spacing (complements Tailwind's 0.5=2px, 1=4px, 1.5=6px, 2=8px, 2.5=10px, 3=12px, 3.5=14px, 4=16px, 5=20px, 6=24px)
    spacing: {
      "0.75": "3px",  // fine-tune offset
      "1.25": "5px",  // compact chip padding
      "1.75": "7px",  // compact row padding
      "2.25": "9px",  // mid-step padding
      "2.75": "11px", // mid-step padding
      "4.5":  "18px", // between p-4 (16px) and p-5 (20px)
      "5.5":  "22px", // between p-5 (20px) and p-6 (24px)
      "6.5":  "26px", // between p-6 (24px) and p-7 (28px)
      "7.5":  "30px", // between p-7 (28px) and p-8 (32px)
      "8.5":  "34px", // between p-8 (32px) and p-9 (36px)
    },
    // Custom animation durations (complements Tailwind's 75/100/150/200/300/500/700/1000ms)
    transitionDuration: {
      120: "120ms",
      140: "140ms",
    },
  },
});
