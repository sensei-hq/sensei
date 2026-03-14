---
id: smart-context-delivery
type: feature
---

# Smart Context Delivery

> Your agent gets exactly what it needs for the current task, nothing more

Agents default to reading whole files and loading broad context, which burns token budget on code that has nothing to do with the task at hand. Sensei's context pipeline ranks, slices, and assembles a tight context pack: only the symbols that matter, trimmed to relevant line ranges, deduplicated against what the agent already read this session, and capped to a hard token budget.

## Features

### Automatic Context Assembly

The automatic context assembly tool accepts a task description and an active file path, and returns rendered markdown, a token count, and assembly stats. It is distinct from explicit context loading, which is agent-chosen: automatic context assembly is task-driven and requires no agent judgment about what to load.

```gherkin
Feature: Automatic Context Assembly

  Scenario: Agent requests context for a task
    Given an indexed repo
    When the agent requests context for "add retry logic to the payment processor" with an active payments file
    Then rendered markdown is returned with a token count and pipeline stats
    And the content is focused on payment-related symbols and their dependencies

  Scenario: Context assembly respects the token budget
    Given an indexed repo with a configured token budget of 8000
    When the agent requests context assembly for a task that touches many files
    Then the rendered markdown is within the 8000 token limit
    And the stats indicate how many lower-ranked symbols were dropped to fit

  Scenario: Context assembly includes git diff context
    Given a repo where three files have been modified in the current branch
    When the agent requests context assembly for a task
    Then the git diff for those three files is included in the result
    And the diff section does not exceed 20% of the total token budget

  Scenario: Automatic assembly is distinct from explicit context loading
    Given an agent that has already explicitly loaded an auth file at a chosen resolution level
    When the agent requests automatic context assembly for "add rate limiting to auth endpoints"
    Then context is assembled automatically based on the task
    And does not require the agent to specify which files or symbols to load
```

### Diff-First BFS Ranking

The ranking stage seeds a breadth-first traversal from the most task-relevant entry points. Files in the current git diff receive a higher base score; the active file also receives a boost. Neighbors decay in relevance with each hop through the call graph. The ranking strategy chain is configurable and can include diff-first traversal, traceability boosting, external docs, semantic search, and keyword-based ranking.

```gherkin
Feature: Diff-First BFS Ranking

  Scenario: Files in git diff are ranked highest
    Given a repo where src/payments/charge.ts and src/payments/retry.ts have uncommitted changes
    When the ranking stage runs for a task about payment processing
    Then both changed files receive a boosted base score
    And they appear at the top of the ranked file list

  Scenario: BFS neighbors decay by hop distance
    Given an indexed repo where charge.ts calls validateCard.ts and logTransaction.ts
    When BFS ranking traverses one hop from charge.ts
    Then validateCard.ts and logTransaction.ts each receive a lower score than charge.ts
    And files two hops away receive a further reduced score

  Scenario: Traceability boost elevates requirement-linked files
    Given a repo with a traceability link from requirement REQ-42 to src/billing/invoice.ts
    And the current task description references REQ-42
    When the ranking stage applies the traceability_boost strategy
    Then src/billing/invoice.ts receives an additional score boost
    And it ranks above unlinked files with the same BFS distance

  Scenario: Semantic fallback activates for tasks with no diff signal
    Given a repo with no uncommitted changes
    When the agent requests context assembly for "add input validation to the user registration flow"
    Then the ranking stage falls through to the semantic strategy
    And files are ranked by embedding similarity to the task description
```

### AST Symbol Slicing

Instead of including full files, the slicing stage extracts only the function and class bodies relevant to the task, using line ranges from the symbol index. Symbols are scored by task keyword match and call-graph proximity. The highest-scoring symbols are selected; adjacent line ranges with a small gap are merged into a single slice.

```gherkin
Feature: AST Symbol Slicing

  Scenario: Only relevant functions are sliced from a large file
    Given a 350-line file containing 12 functions
    And 3 of those functions are relevant to the current task
    When the slicing stage runs
    Then only those 3 function bodies are included in the assembled context
    And the total contribution from that file is 40-80 lines, not 350

  Scenario: Adjacent slices are merged
    Given two relevant functions at lines 120-145 and lines 147-170 in the same file
    When the slicing stage processes that file
    Then the two ranges are merged into a single slice from lines 120-170
    And no redundant gap lines are emitted between them

  Scenario: Symbols are scored by task keyword and call-graph proximity
    Given a task description containing the keywords "invoice" and "tax"
    And symbols named calculateTax (direct keyword match) and renderInvoice (keyword match, 1 hop away)
    When the slicing stage scores symbols
    Then calculateTax ranks higher due to direct keyword match
    And both symbols are included within budget

  Scenario: Slicing respects the code slice budget allocation
    Given an assembled context with 8000 total tokens
    And 1600 tokens already allocated to header and git diff
    When the slicing stage selects code slices
    Then code slices fill at most the remaining 6400 tokens
    And slices are added in descending rank order until the budget is exhausted
```

### Session Deduplication

Sensei tracks every file the agent has read in the current session. When assembling a context pack, files already read and unchanged since they were read are skipped. This typically saves 15–30% of tokens on turn 2 and beyond.

```gherkin
Feature: Session Deduplication

  Scenario: Previously read files are skipped in subsequent context assembly
    Given an agent that read an auth session file on turn 1
    And that file has not changed since it was read
    When the agent requests context assembly on turn 2
    Then the file is not included in the assembled context
    And the stats show it as deduped

  Scenario: Changed files are re-included even if previously read
    Given an agent that read an auth session file on turn 1
    And that file has been modified since turn 1
    When the agent requests context assembly on turn 2
    Then the file is re-included in the assembled context
    And the stats show it was re-included due to change detection

  Scenario: File reads are recorded automatically
    Given an agent that reads a file directly
    When the read completes
    Then the file is recorded as read for the current session
    And the record includes the session, file path, and timestamp

  Scenario: Deduplication saves tokens on multi-turn sessions
    Given a session where the agent read 20 files on turns 1 and 2
    And none of those files have changed
    When the agent requests context assembly on turn 3
    Then all 20 previously read files are excluded
    And the resulting context is at least 15% smaller than it would be without deduplication
```

### Token Budget Management

Each context assembly call operates within a hard token budget (default 8K, configurable per repo). Token counting is model-aware. Budget is allocated proportionally: a small header, a capped share for git diff, a capped share for external doc content, and code slices filling the remainder in descending rank order. External doc content is subject to its own cap within the overall budget.

```gherkin
Feature: Token Budget Management

  Scenario: Assembled context stays within configured budget
    Given a repo with a token budget configured to 6000
    When the agent requests context assembly for any task
    Then the total token count in the returned context is at most 6000
    And the stats indicate how many symbols were dropped to fit

  Scenario: Budget allocation follows defined proportions
    Given a default 8000-token budget
    When the assembler constructs the context with git diff, doc sections, and code slices
    Then the git diff section consumes at most 1600 tokens (20%)
    And doc sections consume at most 1600 tokens (20%)
    And code slices fill the remaining budget

  Scenario: Token counter is model-aware
    Given a session using a specific model
    When the token counter counts tokens for a candidate slice
    Then the count matches the model's actual tokenization

  Scenario: External doc content is capped
    Given a task where three external documentation pages are relevant
    When the assembler includes external doc content
    Then external doc content does not exceed 15% of the total token budget
    And the highest-ranked external content is selected first within that cap
```

### Explicit Context Loading and Next-Step Recommendations

Explicit context loading gives the agent direct control to load a specific file or module at a chosen resolution level: L0 (signatures only), L1 (signatures + docstrings), L2 (logic flow), or L3 (full source). Next-step recommendations accept a task description and prescribe which scope and resolution level the agent should load next, reducing guesswork about where to look.

```gherkin
Feature: Explicit Context Loading and Next-Step Recommendations

  Scenario: Agent loads a module at L0 for cheap discovery
    Given an indexed repo
    When the agent loads a payments module directory at L0
    Then only exported function signatures are returned for all files in that module
    And no function bodies or docstrings are included
    And token usage is under 200 tokens for a typical module

  Scenario: Agent loads a single file at L3 for editing
    Given an indexed repo
    When the agent loads a specific payments file at L3
    Then the full source of that file is returned
    And the response includes all functions, types, and imports

  Scenario: Next-step recommendation prescribes scope and level for a task
    Given an indexed repo
    And the agent is working on "add webhook signature validation to the payments module"
    When the agent requests a next-step recommendation for that task
    Then the response prescribes a specific scope and a resolution level
    And a rationale explaining why that scope and level are sufficient

  Scenario: Next-step recommendation avoids already-read files
    Given an agent that has already loaded a payments file at L3
    When the agent requests a next-step recommendation for a related payments task
    Then the already-loaded file is not recommended again
    And the prescription focuses on files not yet in context
```

### Answer-Driven Relevance Learning

After each agent response, Sensei identifies which symbols were cited or used. Hit and miss rates for symbols are updated and persisted. Future ranking calls apply learned score multipliers, reducing the rank of consistently uncited symbols and elevating consistently cited ones, making context assembly more accurate with each session.

```gherkin
Feature: Answer-Driven Relevance Learning

  Scenario: Cited symbols receive a positive relevance signal
    Given a session where the agent's response cited validatePayment and chargeCard
    When the relevance learning stage processes the response
    Then a positive relevance signal is recorded for validatePayment and chargeCard
    And their ranking multipliers are increased

  Scenario: Loaded but uncited symbols receive a negative signal
    Given a session where tokenizeCard was included in the assembled context but never mentioned in responses
    When the relevance learning stage processes the session
    Then a negative relevance signal is recorded for tokenizeCard
    And its ranking multiplier is decreased

  Scenario: Learned multipliers affect future ranking
    Given that validatePayment has accumulated a strong positive multiplier over 10 sessions
    When context is assembled for a payments task
    Then validatePayment's score is boosted by the learned multiplier
    And it appears higher in the ranked list than symbols with no learning signal

  Scenario: Relevance learning improves across sessions
    Given a repo where context has been assembled 20 times across multiple sessions
    When the agent requests context assembly for a familiar task type
    Then the assembled context omits symbols that were consistently not cited for similar tasks
    And includes symbols that were consistently cited
```
