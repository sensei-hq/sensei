# Tasks: add-dojo-repo-screen (#532)

TDD: red test first for each item.

- [x] 1. Metrics route: tenant member read over
      `repository_metric_series`; 404 outside the tenant; route specs.
- [x] 2. Shared repo loader context (org+repo+branches); both pages use
      it; loader specs.
- [x] 3. Page: header, metric cards, diagrams link; honest empty/error
      states; specs. (Shared `RepoMetricCards`, extracted from the
      workspace preview.)
- [x] 4. Gates green, commit, push. (Dojo 2549, svelte-check clean.)
