# Sensei Site · UI kit

The marketing website — sentinel example of the "Same World" direction. Quieter than a typical SaaS site by design: one column, generous whitespace, sumi ink on washi paper, vermillion rationed to brand and CTAs.

This kit recreates Variant A from the source prototype (`sensei_src/site/variant-a.jsx`), rebuilt against the rationalized Zen-Sumi system.

## Files

- `index.html` — the full marketing page (run it)
- `Nav.jsx` — `<Nav/>`: brand mark + thin top nav
- `Hero.jsx` — `<Hero/>`: kanji + eyebrow + headline + lead + CTA + product mock
- `Section.jsx` — `<Section/>`: a generic two-column eyebrow/title + body section
- `HowItWorks.jsx` — `<HowItWorks/>`: the Watch · Notice · Adopt three-column block
- `Philosophy.jsx` — `<Philosophy/>`: centered single-kanji statement
- `Faq.jsx` — `<Faq/>`: hairline-divided expandable Q&A
- `Footer.jsx` — `<Footer/>`: brand mark + version + links
- `Mock.jsx` — `<MockToday/>`: a tiny stylised product screenshot used in the hero

## Notes

The marketing site does NOT use the Tauri window chrome — that's app-only. Site uses a narrower content column (1100px max), heavier vertical rhythm (`--space-7` between sections), and the largest type sizes (`--text-3xl`, `--text-4xl`).

CTAs auto-detect the user's OS (the `<DownloadCTA/>` component). Pricing has no plans; there is one button, "Download for [your OS]".
