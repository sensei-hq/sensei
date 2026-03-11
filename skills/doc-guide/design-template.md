---
id: <component-name>
type: design
implements:
  - feature: <feature-id>
    items: [<item-id>, <item-id>]
---

# <Component Name>

## Overview

[2–3 sentences: what this component does, its role in the system, and the key design decision that shapes everything else.]

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| <nfr-name> | <acceptance criterion — measurable> |

---

## <Data Model / Schema / File Layout>

[File layouts, data schemas, type definitions, storage formats.]

```
example/
  structure.yaml     ← purpose
  file.ts            ← purpose
```

---

## <Algorithm / Flow / Protocol>

[Sequences, decision logic, state machines. Use pseudocode or numbered steps.]

```
Step 1: <action>
  → sub-step
Step 2: <action>
```

---

## <API / Tool Contracts>

[Function signatures, MCP tool contracts, CLI command specs.]

```typescript
functionName(param: Type): ReturnType
// param: description
// returns: description
// throws: ErrorType when X
```

---

## Error Handling

```
Missing X:  "X not found. Run Y first."
Invalid Y:  "Y is invalid. Expected Z."
```

---

## Testing Strategy

```
Unit: src/<module>/<component>.spec.ts
  - <key test case>
  - <key test case>

E2E: e2e/<feature>.e2e.ts
  - <key integration scenario>
```

---

## Open Questions

| Question | Status |
|----------|--------|
| <question> | 🔲 Open |

---

> This is a **design doc** — how it works, not what it does.
> User-facing needs belong in `docs/features/`.
> Status lives in `docs/traceability.yaml` — do not add a status table here.
