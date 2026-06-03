# Zen-Sumi kanji reference

The functional kanji used throughout the Sensei product. These are not decoration —
each one has a fixed meaning. When in doubt, use a sentence; do not invent a new kanji.

## Brand

| Kanji | Romaji | Meaning |
|---|---|---|
| 先生 | sensei | teacher / master — the brand mark |

## Phases of practice (the product's narrative spine)

| Kanji | Romaji | Phase |
|---|---|---|
| 観 | kan | Watch / observe |
| 察 | satsu | Notice / discern |
| 覚 | kaku | Adopt / remember |

## Navigation / sections

| Kanji | Romaji | Section |
|---|---|---|
| 場 | ba | Projects |
| 刻 | koku | Sessions |
| 具 | gu | Instruments / Tools |
| 蔵 | kura | Archive / Privacy |

## States

| Kanji | Romaji | State |
|---|---|---|
| 動 | dō | Active |
| 眠 | nemuri | Dormant |
| 旧 | kyū | Recent / old |
| 空 | kū | Empty |
| 試 | kokoromi | Test / try |
| 探 | saguri | Search |
| 静 | sei | Stillness |

## Project-specific (examples)

| Kanji | Romaji | Project |
|---|---|---|
| 工 | kō | Lumen Studio (craft) |
| 雲 | kumo | Lumen Cloud |
| 紋 | mon | Brand Kit (crest) |
| 筆 | fude | Sketch tool (brush) |
| 巻 | maki | Docs (scroll) |

## Rendering

Always wrap in `<span class="zs-kanji">`:

```html
<span class="zs-kanji" style="font-size: var(--text-2xl); color: var(--accent);">
  観
</span>
```

The CSS class swaps to `--font-kanji` (Yu Mincho → Hiragino Mincho ProN → Songti SC → serif).
Latin glyphs in the same string will be rendered in this serif too, which is usually wrong —
keep kanji and Latin in separate spans.

## Coloring

- `--accent` (vermillion) for functional, *do-this* kanji in active surfaces.
- `--ink-3` for decorative or label kanji.
- `--ink-4` for disabled or empty-state kanji.
- Never use a kanji at less than ~14px — the brush detail dies. Below that, use a sentence or a dot.
