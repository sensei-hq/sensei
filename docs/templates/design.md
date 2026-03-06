# [Component Name]

## Overview

[2-3 sentences: what this component does, its role in the system, and the key design decision that shapes everything else.]

---

## [Structure / Schema / Data Model]

[Use this section for file layouts, data schemas, type definitions, storage formats. Use code blocks and tables.]

```
example/
  structure.yaml     ← purpose
  file.ts            ← purpose
```

---

## [Algorithm / Flow / Protocol]

[Use this section for sequences, decision logic, state machines. Use diagrams, pseudocode, or numbered steps.]

```
Step 1: [action]
  → sub-step
Step 2: [action]
  → sub-step
```

---

## [API / Tool Contracts]

[Use this section for function signatures, MCP tool contracts, CLI command specs.]

```typescript
functionName(param: Type): ReturnType

// param: description
// returns: description
// throws: when X happens
```

---

## [Configuration]

[Use this section for config file formats, environment variables, flags.]

```yaml
field: value    # description
```

---

## [Error Handling]

[How errors surface. What messages are returned. What the caller should do.]

```
Missing X: "X not found. Run Y first."
Invalid Y: "Y is invalid. Expected Z."
```

---

## [Testing Strategy]

[How this component is tested. Key test cases. What fixtures are used.]

```
Unit: src/[module].spec.ts — tests against temp dirs in /tmp/
E2E:  e2e/[feature].e2e.ts — full CLI invocation tests
```

---

## Open Questions

| Question | Status |
|---|---|
| [Question] | 🔲 Open |
| [Question] | ✅ Resolved: [answer] |

---

## Notes:

> This is a **design doc** — how it works, not what it does.
> User-facing needs belong in `docs/features/`.
> Notes section is for guidance only not needed in actual docs.
