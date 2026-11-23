# Tasks: add-relay-launch-templates (#513)

TDD: each item starts with a red test before the green code. DDL first in
`database/ddl`, applied with `dbd`; seed rows via the staging import.

- [x] 1. DDL: `launch_template text null` on `sensei.assistant_catalogue`
      (full-table file + comment), staging table, import procedure, seed
      values in `assistant_catalogue.jsonl` (null where unsure)
- [x] 2. Daemon: resolve `{repo}` (crew repo, else workspace first) and
      serve `launch` per item from `GET /relay/catalogue`
      (`workspace_id` + `crew_id` query); tests for both repo paths,
      null template, and no-repo
- [x] 3. App: `launch` on the catalogue item view, picked tool's block
      under the paste line with Copy; specs for render/copy/absent
- [x] 4. Gates green (unit, check, senseid, clippy, fmt), e2e relay crew
      spec still green, commit, push
