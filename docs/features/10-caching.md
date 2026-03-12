---
id: caching
type: feature
---

# Caching

Claude frequently provides valuable analysis — a comparison of three approaches, a list of options with trade-offs, a detailed explanation of a design decision. These responses disappear when the session ends. The next session starts from blank state, prompting the same question and spending the same tokens to re-derive the same answer. Response caching captures these outputs at the moment they're produced and makes them retrievable across sessions by semantic query.

## Features

### Response Capture

Notable Claude outputs are saved with enough context to be useful when retrieved later.

```gherkin
Feature: Response Capture

  Scenario: Agent captures its own response on request
    Given a Claude response containing a comparison of three indexing approaches
    When the user says "remember this" or the agent calls cache_response(label, content)
    Then the response is saved to .index/response-cache/
    And it is tagged with: timestamp, session ID, topic, and optional user-supplied label

  Scenario: Agent proactively offers to cache a significant response
    Given a response that contains a structured comparison, ranked options, or design analysis
    When the agent finishes generating the response
    Then the agent appends: "Want me to cache this for future sessions? (say 'cache it')"
    And waits for user confirmation before saving

  Scenario: Cached response includes retrieval context
    Given a cached response about indexing approach trade-offs
    Then the cache entry includes: the original question/prompt that generated it
    And the topic tags derived from the content
    And a one-line summary for display in search results
```

### Response Retrieval

Cached responses are retrievable by semantic query without loading everything into context.

```gherkin
Feature: Response Retrieval

  Scenario: Agent retrieves a cached response by query
    Given a cached response tagged "indexing approach comparison"
    When the agent calls find_cached_response("which indexing approach did we evaluate")
    Then the most relevant cached response is returned
    And token usage reflects only the matched response, not the full cache

  Scenario: Agent surfaces relevant cache at session start
    Given cached responses relevant to the current task
    When the agent calls get_session_context()
    Then the response includes a "Cached context" section
    And lists up to 3 relevant cached items with one-line summaries
    And the agent can call find_cached_response() to load any of them in full

  Scenario: No matching cached response
    Given a query that doesn't match any cached response
    When the agent calls find_cached_response("unknown topic")
    Then the response says no cached response found
    And does not error
```

### Cache Management

The cache stays useful without growing unbounded.

```gherkin
Feature: Cache Management

  Scenario: Cached responses have a default TTL
    Given a cached response older than 90 days that has never been retrieved
    When the agent runs cache maintenance
    Then the stale response is moved to archive
    And no longer appears in search results

  Scenario: Retrieved responses get TTL extended
    Given a cached response that was retrieved 3 times in the last 30 days
    When TTL maintenance runs
    Then the response TTL is extended by 90 days
    And it is not archived

  Scenario: User can pin a response permanently
    Given a cached response the user wants to keep indefinitely
    When the user runs sensei cache pin <id>
    Then the response is marked pinned
    And TTL maintenance never archives it

  Scenario: User can delete a cached response
    Given a cached response that is no longer relevant
    When the user runs sensei cache delete <id>
    Then the response is removed from the cache
    And it no longer appears in search results

  Scenario: Cache summary is available
    Given 12 cached responses across 4 topics
    When the agent calls list_cached_responses()
    Then a summary is returned with count, topics, and oldest/newest dates
    And no full response content is loaded
```

### Integration with Session Resume

Relevant cached responses are surfaced passively at session start.

```gherkin
Feature: Integration with Session Resume

  Scenario: Session context includes cache hints
    Given cached responses related to the active task area
    When the agent calls get_session_context()
    Then the response lists up to 3 relevant cached items
    And each item shows: label, one-line summary, and retrieval command

  Scenario: Cache hints don't bloat session context
    Given 50 cached responses in the project
    When get_session_context() is called
    Then at most 3 cache hints are included
    And total session context remains under 500 tokens

  Scenario: Agent can request cache search separately
    Given a session where cached context isn't surfaced automatically
    When the agent calls find_cached_response("trade-offs we discussed for auth")
    Then the relevant cached response is returned
    And the agent can load it into context on demand
```
