# Benchmarking

Skills and MCP tools are only valuable if their impact is measurable. Benchmarking provides a structured way to compare agent performance with and without skills, across a representative set of developer tasks, and to track improvement over time.

## Features

### Task Corpus

A curated set of representative developer tasks that covers the range of work agents do in real codebases.

```gherkin
Feature: Task Corpus

  Scenario: Corpus covers all developer task categories
    Given the sample task corpus
    Then it contains at least one task in each category:
      orientation, discovery, understanding, feature-add, bug-fix, refactor, doc-update, drift-check

  Scenario: Each task has a clear success criterion
    Given any task in the corpus
    Then it has a prompt field describing the task
    And a success_criteria field describing what a correct response looks like
    And a category field for grouping results

  Scenario: Corpus is extensible
    Given the tasks/sample.yaml file
    When a developer adds a new task entry following the schema
    Then the benchmark runner picks it up automatically on the next run
```

### A/B Evaluation

The same tasks run against two configurations — with skills and without — and results are compared side by side.

```gherkin
Feature: A/B Evaluation

  Scenario: Benchmark runs against both configurations
    Given a task corpus and two configured branches (with-skills, without-skills)
    When the benchmark is run
    Then each task executes once in the with-skills configuration
    And once in the without-skills configuration
    And results are stored separately for comparison

  Scenario: Without-skills baseline is a clean environment
    Given the without-skills configuration
    Then no .index/ directory is present
    And no skills are loaded
    And the MCP server is not running
    And the agent has only the raw repo

  Scenario: With-skills configuration is fully enabled
    Given the with-skills configuration
    Then the repo is indexed
    And skills are loaded via ~/.claude/skills
    And the MCP server is running
    And .llmspec.yaml and llms.txt are present
```

### Metrics Collection

Five metrics are captured per task per configuration.

```gherkin
Feature: Metrics Collection

  Scenario: Token counts are captured
    Given a benchmark task run
    Then the result records tokens_in (prompt tokens consumed)
    And tokens_out (completion tokens generated)

  Scenario: Interaction count is captured
    Given a benchmark task run
    Then the result records the number of turns taken to complete the task

  Scenario: Tool call count is captured
    Given a benchmark task run
    Then the result records how many tool calls were made
    And distinguishes between MCP tool calls and file read calls

  Scenario: Success is recorded
    Given a benchmark task run
    Then the result records whether the task was completed successfully
    And whether the output matches the success criteria

  Scenario: Drift score is recorded for doc tasks
    Given a benchmark task in the doc-update or drift-check category
    Then the result records the drift score before and after the task
```

### Results Comparison

Benchmark results are stored as JSON and compared to produce a human-readable summary.

```gherkin
Feature: Results Comparison

  Scenario: Results are stored as structured JSON
    Given a completed benchmark run
    Then results are written to results/YYYY-MM-DD-benchmark.json
    And the file contains per-task metrics for both configurations

  Scenario: Comparison report highlights differences
    Given two benchmark result files (with-skills and without-skills)
    When the agent calls compare_results(file_a, file_b)
    Then the report shows percentage change in tokens_in, tokens_out, interactions, and tool_calls
    And highlights tasks where the improvement is largest
    And flags tasks where the with-skills performance was worse

  Scenario: Summary committed to results directory
    Given a completed benchmark comparison
    Then a human-readable summary is committed to results/
    And raw JSON files are gitignored to avoid bloating the repo

  Scenario: Trend visible across multiple runs
    Given multiple benchmark runs over time
    When the agent calls get_metrics_summary()
    Then the trend in key metrics is shown
    And improvements (or regressions) since the first run are highlighted
```

### Improvement Loop

Benchmarks feed directly back into skill improvement.

```gherkin
Feature: Improvement Loop

  Scenario: Weak categories are identifiable from results
    Given a benchmark comparison report
    Then tasks are grouped by category
    And the average improvement per category is shown
    And categories with low improvement (< 20%) are highlighted

  Scenario: Specific skill can be targeted for improvement
    Given a category with low benchmark improvement
    When the developer reviews the relevant skill
    Then the skill can be updated to address the identified gap
    And a re-run of the benchmark confirms the improvement

  Scenario: Regression is caught
    Given a benchmark baseline
    And a skill has been modified
    When the benchmark is re-run
    Then any metric that has regressed compared to the baseline is flagged
    And the comparison report marks the regression clearly
```

## Status

| Feature | Status |
|---------|--------|
| Task corpus (tasks/sample.yaml) | 🔲 Planned |
| Task categories covering all developer work | 🔲 Planned |
| A/B evaluation setup | 🔲 Planned |
| Token count metrics | 🔲 Planned |
| Interaction count metrics | 🔲 Planned |
| Tool call metrics | 🔲 Planned |
| Success criteria evaluation | 🔲 Planned |
| Results storage as JSON | 🔲 Planned |
| Comparison report (compare_results MCP tool) | 🔲 Planned |
| Metrics summary and trend (get_metrics_summary) | 🔲 Planned |
| Improvement loop documentation | 🔲 Planned |
