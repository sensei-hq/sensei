## 1. Ack dispositions (test-first)

- [ ] 1.1 DDL `sensei.alert_acks` (full-table file, `dbd` deploy, test-template rebuild)
- [ ] 1.2 `GET`/`PUT /api/ops/alerts/acks` incl. 90-day stale prune in PUT
- [ ] 1.3 Alerts store hydrates on attach, writes through on ack/clear
- [ ] 1.4 State + endpoint specs; live-verified on scratch daemon

## 2. Wave history (test-first)

- [ ] 2.1 DDL `activity.wave_runs` (full-table file, `dbd` deploy, test-template rebuild)
- [ ] 2.2 Totals opens the row, terminal closes it (`done`); boot closes leftovers (`abandoned`)
- [ ] 2.3 `GET /api/tasks/waves` (filterable, capped, exact totals)
- [ ] 2.4 Pipelines/Sync sections render history beside live waves; rendered specs
- [ ] 2.5 Live-verified on scratch daemon (run/inspect/abandon); Tauri matrix in slice 4

## 3. Boot re-drive (test-first)

- [ ] 3.1 Transcript re-drive: fresh backfill covering un-ingested sessions, no duplicates
- [ ] 3.2 Metric re-drive: missed days via the `as_of` path, no grain gaps
- [ ] 3.3 Sync: assert next-pass coverage, no boot pass (push-ack race documented in code)
- [ ] 3.4 Index resume pinned by kill-restart proof (fingerprint counts, no redo)
- [ ] 3.5 Schedule self-heal asserted sufficient (no new machinery)

## 4. Kill-restart e2e matrix + full gates (Tauri Playwright, live)

- [ ] 4.1 Index crash resumes without redo
- [ ] 4.2 Wave crash abandons and re-drives
- [ ] 4.3 Acks survive reload; recurrence escalates across restart
- [ ] 4.4 Full unit + e2e + senseid suites green; clippy/fmt/fabrication clean; commit
