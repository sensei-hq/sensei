---
id: resolution-levels
type: design
implements:
  - feature: resolution
    items: [resolution-levels, docstring-stripping, logic-flow-notation, io-pattern-notation, task-to-level-mapping]
---

# Content Compression

## Overview

Code is stored at four resolution levels in the symbol map. The MCP server serves the requested level. Agents choose the right level based on task type — guided by the `content-compression` skill and the `recommend_next` tool.

---

## Resolution Level Definitions

### L0 — Signature

The exported declaration line, stripped of body, imports, and doc-comments.

**TypeScript example:**
```
export async function processOrder(orderId: string, options?: ProcessOptions): Promise<Order>
export class OrderService
export type OrderStatus = "pending" | "processing" | "complete" | "cancelled"
export const MAX_RETRIES = 3
```

**Rules:**
- Remove opening brace and everything after
- Remove JSDoc above the declaration
- Remove decorator lines (`@Injectable()`, `@Controller()`, etc.)
- Keep: `export`, access modifiers, function name, parameters with types, return type

### L1 — IO Pattern

Assignment notation showing what goes in and what comes out. No implementation details.

**Format:**
```
result = functionName(param1, param2)
// param1: type
// param2: type (optional)
// returns: ReturnType
```

**TypeScript example** for `processOrder`:
```
order = processOrder(orderId, options?)
// orderId: string
// options: ProcessOptions (optional)
// returns: Promise<Order>
```

**Rules:**
- No async/await notation
- `?` suffix for optional parameters
- Return type always shown
- `void` or `→ side effect only` for void functions
- Constructor shown as: `service = new OrderService(deps)`

### L2 — Logic Flow

Plain-English description of what the function does, in the minimal form that conveys the logic. V1 is a placeholder; V2 uses LLM summarisation.

**Notation rules:**

| Construct | Notation |
|---|---|
| Sequential steps | `step1 → step2 → output` |
| Conditional | `if valid → proceed` / `else → throw ValidationError` |
| Loop | `for each item → transform and collect` |
| Async | `await fetch → check status → parse` |
| State transition | `pending → processing → complete \| failed` |
| Early return | `if missing → return null` |

**Example** for `processOrder`:
```
validate orderId → fetch order from DB
if order not found → throw NotFoundError
charge payment → if charge fails → rollback, throw PaymentError
update order status to "processing" → emit order.processed event → return order
```

**Rules:**
- No code syntax (no `if`, `return`, `const`, etc.)
- No type annotations
- 3–8 lines maximum
- Each line is one logical step

### L3 — Full Source

The actual file content, read directly from disk. Not stored in `symbol-map.json` — always read live.

---

## Storage Schema

`symbol-map.json` stores L0, L1, and L2 arrays per file. Each array index corresponds to the same export (parallel arrays).

```json
{
  "src/orders.ts": {
    "L0": [
      "export async function processOrder(orderId: string): Promise<Order>",
      "export function cancelOrder(orderId: string, reason: string): Promise<void>"
    ],
    "L1": [
      "order = processOrder(orderId)\n// orderId: string\n// returns: Promise<Order>",
      "cancelOrder(orderId, reason)\n// orderId: string\n// reason: string\n// returns: void"
    ],
    "L2": [
      "validate orderId → fetch order → charge payment → update status → emit event → return order",
      "// L2 not yet generated"
    ]
  }
}
```

L3 is never in `symbol-map.json`. `get_file_context(path, "L3")` reads the file from disk.

---

## Serving Logic

`get_file_context(path, level)` in `tools/query.ts`:

```
L0/L1/L2:
  1. Read symbol-map.json
  2. Find entry for path
  3. Return the array joined with "\n"
  4. If path not in map: error "File not in symbol map. Run reindex_repo first."

L3:
  1. Read file from disk at REPO_PATH/path
  2. Return full content
  3. If file not found: fs error propagated with context
```

---

## Docstring Stripping Rules

Applied during indexing when building L0/L1 from source.

**Strip from L0 and L1:**
- JSDoc blocks (`/** ... */`)
- Single-line doc comments (`// Description of function`)
- Python docstrings (`"""..."""`)
- Go doc comments (`// FunctionName does...`)
- Decorator lines that are doc-related (`@deprecated`, `@param`, `@returns`)

**Keep in L0:**
- Decorators that are structural (`@Injectable`, `@Controller`, `@Get`)
- Access modifiers (`public`, `private`, `protected`)
- Type annotations and generics

**Keep in L3:**
- Everything, unmodified

---

## Token Cost Estimates

Based on typical TypeScript codebases:

| Level | Tokens per function | Tokens for 20-function module |
|---|---|---|
| L0 | 8–15 | 150–300 |
| L1 | 20–40 | 400–800 |
| L2 | 30–80 | 600–1600 |
| L3 | 100–500+ per function | 2000–10000+ |

L3 token cost is unbounded — depends on function size. Always prefer L2 or lower unless editing.
