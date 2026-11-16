## 1. Guard first
- [x] 1.1 The layout guard (app; the dōjō's with its §4.7 fixes) + allowlist of today's offenders (count only shrinks); in the pre-commit fast suite

## 2. rokkit (red first, released)
- [x] 2.1 screen-shell.css: mobile-first info band (collapsed on phone) and stats (2-up phone); `min-width` steps
- [x] 2.2 stat-card.css: the label wraps; rokkit patch release; app + dōjō bumped

## 3. Shells
- [x] 3.1 Rail → drawer below md: (observatory, workspace, settings layouts); header menu button (unit-pinned; the 430px e2e pass is 5.1)
- [ ] 3.2 Setup wizard rail stacks on phone

## 4. Screens (worst first; each: red spec or phone capture, then the fix; allowlist shrinks)
- [x] 4.1 Relay: LaneRow name `flex-[1_1_0] min-w-0`; RunCard / RelayRunScreen facts wrap
- [ ] 4.2 Observatory home, RecentSessions; instruments; impact; consolidation; insights; workspaces; learnings; libraries; knowledge sources
- [ ] 4.3 Activity logs + health logs (overflow wrappers / stacked rows)
- [ ] 4.4 Workspace impact, memories, overview, instruments, about
- [ ] 4.5 Settings general, metrics, inference, instruments; page padding `px-4 md:px-10`
- [ ] 4.6 Setup scan, welcome, done
- [ ] 4.7 dōjō RelayDetail padding, DojoSignIn, OrgSwitcher width

## 5. Verification
- [ ] 5.1 `/preview` capture at phone for Relay; a 430px e2e pass over the main windows (no page h-scroll)
- [ ] 5.2 `openspec validate update-phone-layout --strict`
