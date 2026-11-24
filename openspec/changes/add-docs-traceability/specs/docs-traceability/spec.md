# docs-traceability

## ADDED Requirements

### Requirement: Restructured-text and asciidoc index as documents
The processor router SHALL send `rst` and `adoc` files to the doc
processor, completing the `file_role` docs convention. No other routing
SHALL change.

#### Scenario: An .rst file decomposes
- **GIVEN** a checkout containing `guide.rst` with two headings
- **WHEN** it is indexed
- **THEN** its sections land as `section` nodes like any markdown file's

### Requirement: Doc links resolve to references edges
The indexer SHALL turn a doc link (markdown, `rst`, or `adoc`) whose
target is a file in the same checkout into a `references` edge from the
doc's node to that file's node — the D2 path and its kind. A link whose
target is outside the checkout SHALL become a named-unresolved
`references` edge, never dropped and never an edge to nothing.
Re-indexing SHALL reconcile the edge set (a removed link prunes).

#### Scenario: A doc documents its module
- **GIVEN** `shop/README.md` linking `shop/cart.ts`
- **WHEN** the checkout is indexed
- **THEN** a `references` edge runs from the README's node to the file's

#### Scenario: An external link stays named
- **GIVEN** a doc linking `https://example.com/x`
- **WHEN** it is indexed
- **THEN** an unresolved `references` edge carries the URL as its target name

### Requirement: Function mentions resolve only when unambiguous
A `fn_mention` SHALL resolve to its symbol's node only when the name is
the sole definition in the folder; an ambiguous or absent name SHALL
stay a named-unresolved `references` edge, never a fabricated edge.

#### Scenario: A mention of an unknown symbol
- **GIVEN** a doc mentioning `frobnicate()` with no such symbol indexed
- **WHEN** it is indexed
- **THEN** an unresolved `references` edge carries the mention by name

### Requirement: Docs count with the docs role; the documented shade stays code-only
Docs files and section nodes SHALL flow into the Code map's file and
symbol counts under the `docs` role. The documented-share shade SHALL
keep measuring code declarations only (docstring-based); docs nodes
SHALL NOT dilute it.

#### Scenario: A docs-heavy module
- **GIVEN** a module with one code file and one docs file
- **WHEN** its block is read
- **THEN** roles count both files by role and the documented share
  reflects the code file alone
