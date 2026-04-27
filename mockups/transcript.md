Hi-fi design
Interactive prototype
Pasted text (146 lines)
Claude
I'll design an interactive prototype for "Sensei" — a development session observatory. Before diving in, let me ask some focused questions to get the direction right.
Claude has some questions →
You
Questions answered:
- scope: All 7 pages as a clickable prototype (lower fidelity per page)
- platform: Tauri+Sveltekit
- aesthetic: Zen / sensei — calm, airy, light, Japanese-inspired minimalism with ink accents
- data_density: Decide for me
- hero_metric: Decide for me
- interactions: Switching between solutions in the sidebar, Drilling from a metric → the sessions that caused it, Expanding a session to see its event timeline, Clicking a coaching recommendation → 'apply' flow, Filtering sessions by module / FTR / date, Hovering charts to see values
- coaching_tone: Terse, zen, koan-like — 'The AI does not know your auth.'
- variations: 3 directions side-by-side on a canvas
- sample_data: Invent a richer fictional product (e.g. a fintech, a design tool)
- tweaks: Dark / light mode
Claude
I've got a strong direction. Let me lay out the plan and build.

The design system I'll commit to:

Aesthetic: Zen/sensei — washi-paper backgrounds (warm off-white oklch(0.97 0.008 85)), sumi-ink foreground, a single vermillion (朱) accent oklch(0.58 0.15 35) for signal, jade oklch(0.65 0.08 150) for calm/positive states.
Type: Fraunces for display/numerals (editorial, warm serif), Inter for UI, JetBrains Mono for technical. (Using a serif display because the zen direction calls for it — not a default-aesthetic project.)
Voice: terse, koan-like. "The AI does not know your auth." "Three corrections. One teacher."
Product: fictional — Lumen, a design tool company. Solutions: lumen-studio, lumen-cloud, brand-kit.
Layout: full-bleed Tauri window chrome, left sidebar, main canvas. 3 directions shown side-by-side on a design canvas:
Ma (間) — airy, negative-space-forward, one hero insight at a time
Enso (円相) — circular/orbital data viz, FTR as a brushstroke ring
Shoji (障子) — grid-paned, data-dense but calm, like looking through rice paper
Each direction covers all 7 pages via internal navigation. Dark mode exposed as a Tweak.
Updated todos
Copying starter
Let me quickly check the design-canvas API, then build.
Reading
Got it. Now let me build the data, tokens, and three directions. I'll structure this across several files to keep each under the size limit.
Writing ×2
Now the shared primitives (nav, sparklines, brush-ring) and the three directions. Each direction will be its own JSX file with all 7 pages inside, using a local "router" state.
Writing
Now Direction 1: Ma (間) — airy, negative-space-forward. I'll build a compact prototype that covers all 7 pages via internal nav.
Writing
Now Direction 2: Enso (円相) — circular/orbital viz, FTR rendered as a sumi brushstroke ring.
Writing
Now Direction 3: Shoji (障子) — grid-paned, data-dense but calm, like looking through rice paper.
Writing
Now the main HTML composing all three directions on a design canvas.
Writing, Done, Fork verifier agent
Sensei observatory is live — three zen directions on one canvas:

01 · 間 Ma — negative space, one insight at a time; huge serif FTR number, koan-as-hero.
02 · 円 Enso — FTR rendered as a sumi brushstroke ring, sessions orbit around it.
03 · 障子 Shoji — modular rice-paper panes, data-dense but calm; dashboard grid.
All three cover the full 7-page flow (Overview · Observatory · Sessions · Codebase · Coaching · Configuration · Setup), wired with real interactions: sidebar solution switching, metric-to-sessions drill-in, expandable session timelines, apply-coaching flow, module/outcome filters. Sample data is the fictional Lumen design-tool company. Dark/light mode lives in the Tweaks panel — toggle it from the toolbar.

Pick a direction and I'll deepen it — richer interactions on the Observatory, more detailed Codebase graph, a Sessions firehose view, or a different narrative for the coaching koans.
You
I like the sidebar from Ma & Enso, I assume that wider sidebar from ma can be collapsed to the icon only view in enso.

I like the trendline graph from Ma instead of the arc in Enso. I prefer the layout in Enso.
Shoji is too crowded/
Claude
Got it. I'll merge the best of Ma and Enso into a single refined direction: collapsible sidebar (wide Ma ↔ icon-only Enso), Enso's two-column observatory layout, and a trendline instead of the arc ring.
Reading
I'll build the merged direction by creating a new file based on Enso's layout, swapping the ring for the Ma trendline, and adding a collapsible sidebar.
Writing, Editing, Done, Fork verifier agent
The Refined direction is now the top section of the canvas — above the original three for reference:

Collapsible sidebar — starts wide (Ma style with solution names + FTR readouts + page labels). Click the chevron in the header to collapse to icon-only (Enso style). Click the chevron at the bottom to expand again.
Enso's two-column layout — big left column with the hero metric + sessions, right column with coaching and signals.
Ma's trendline — the 14-day sparkline with area fill replaces the brushstroke ring. Colored jade when up, vermillion when down.
Shoji is kept only for reference below; the density you disliked is unchanged there, not carried forward.
Sessions, Codebase, Coaching, Config, Setup reuse the Enso layouts (which you preferred). Dark/light still lives in Tweaks.

Want me to apply the trendline treatment to the overview cards too (I did), or extend the collapsible sidebar to also drive density elsewhere?
