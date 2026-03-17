---
id: code-explorer
type: feature
---

# Code Explorer

> Navigate your codebase visually — see how files and functions relate, find complexity hotspots, and browse every exported symbol without reading the code first.

Understanding a codebase well enough to work in it safely requires knowing how the pieces fit together. Without tooling, developers and agents spend time tracing imports by hand, guessing at blast radius before a change, and discovering hotspots only after bugs emerge. Code Explorer makes that structure visible: an interactive dependency graph, a complexity heatmap, and a searchable symbol table — all derived from the same index that powers context delivery and search.

## Features

### Code Graph

An interactive dependency graph showing which files and functions depend on which others. Nodes are coloured by kind — function, class, module, interface — so the shape of the codebase is readable at a glance. Clicking any node navigates to the source file and line. Edges show call direction; hovering an edge shows the call site context.

```gherkin
Feature: Code Graph

  Scenario: Developer opens the code graph for a repo
    Given a repo that has been indexed by sensei
    When the developer opens the Code Explorer view in the dashboard
    Then an interactive graph is rendered with a node for each indexed symbol
    And nodes are coloured by kind: function (blue), class (green), module (grey), interface (purple)
    And directed edges show which symbols call or import which others

  Scenario: Developer clicks a node to navigate to source
    Given the code graph is rendered and the developer clicks a function node
    When the click event fires
    Then the source file opens with the cursor positioned at the function definition line
    And the file path and line number are shown in the URL

  Scenario: Developer filters the graph to a single file
    Given the code graph shows the full repo
    When the developer enters a file path in the filter box
    Then only nodes belonging to that file and their direct neighbours are shown
    And all other nodes are hidden from the view
```

---

### Complexity Heatmap

Per-file and per-function cyclomatic complexity scores, rendered as a heatmap. High-complexity files and functions are highlighted in red; low-complexity ones in green. Useful for identifying refactoring targets and understanding which areas of the codebase carry the most risk.

```gherkin
Feature: Complexity Heatmap

  Scenario: Heatmap highlights high-complexity functions
    Given a repo with functions of varying cyclomatic complexity
    When the developer opens the Complexity Heatmap view
    Then each function node is coloured on a green-to-red scale based on its complexity score
    And functions with complexity above the configured threshold are labelled as hotspots

  Scenario: Developer sorts by complexity to find refactoring targets
    Given the complexity heatmap is visible
    When the developer clicks "Sort by complexity (descending)"
    Then the list view re-orders with the highest-complexity functions at the top
    And each entry shows: function name, file path, complexity score, and last-modified date
```

---

### Symbol Explorer

A searchable, filterable table of all exported symbols in the repo. Each row shows the symbol name, file path, kind (function, class, interface, type, constant), line number, and complexity score. Developers can filter by file, kind, or complexity range. Clicking a row opens the source at that line.

```gherkin
Feature: Symbol Explorer

  Scenario: Developer searches for a symbol by name
    Given the Symbol Explorer table is open
    When the developer types a symbol name into the search box
    Then the table filters to rows where the name matches (case-insensitive substring)
    And the file path, kind, line number, and complexity score are shown for each match

  Scenario: Developer filters to exported functions only
    Given the Symbol Explorer is showing all symbols
    When the developer selects kind = "function" and visibility = "exported"
    Then only exported functions are shown
    And the row count updates to reflect the filtered result

  Scenario: Developer opens a symbol in source
    Given the Symbol Explorer shows a matching symbol row
    When the developer clicks the row
    Then the source file opens at the symbol's line number
```

---

## Status

| Feature | Status |
|---------|--------|
| Code Graph | 🔲 Planned |
| Complexity Heatmap | 🔲 Planned |
| Symbol Explorer | 🔲 Planned |
