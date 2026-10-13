# Svelte 5 runes

A contributor's reference for the runes used across `app/`, `dojo/` and
`website/` (Svelte 5). Runes are `$`-prefixed symbols the Svelte compiler
understands: they are not imported, cannot be assigned to variables or passed
as arguments, and are only valid in component scripts and `.svelte.js` /
`.svelte.ts` modules. API checked against `svelte` 5.57's type declarations.

| rune | purpose |
|---|---|
| `$state` | reactive state |
| `$derived` | values computed from state |
| `$effect` | side effects that run when state changes |
| `$props` | a component's inputs |
| `$bindable` | a prop the parent may bind two-way |
| `$inspect` | log state when it changes (development) |
| `$host` | the host element of a custom element |

## `$state`

```svelte
<script>
	let count = $state(0);
</script>

<button onclick={() => count++}>clicks: {count}</button>
```

A `$state` variable is read and written like any variable — no setter API.

**Deep state.** Arrays and plain objects become deeply reactive proxies, so
mutating a property updates the UI:

```js
let todos = $state([{ done: false, text: 'add more todos' }]);
todos[0].done = !todos[0].done;
```

**Classes.** Instances are not proxied; declare reactive fields instead:

```js
class Todo {
	done = $state(false);

	constructor(text) {
		this.text = $state(text);
	}

	reset() {
		this.text = '';
		this.done = false;
	}
}
```

**`$state.raw`** — state that is not deeply reactive; reassign rather than
mutate. Cheaper for large values that are replaced wholesale:

```js
let person = $state.raw({ name: 'Heraclitus', age: 49 });
person = { name: 'Heraclitus', age: 50 };
```

**`$state.snapshot`** — a plain, non-proxied copy of state (for logging,
`structuredClone`, or handing to a library that should not see a proxy).

**`$state.eager`** — the latest value even while the rest of the UI is
suspended on async work (such as data loading):

```svelte
<a href="/" aria-current={$state.eager(pathname) === '/' ? 'page' : null}>home</a>
```

## `$derived`

```svelte
<script>
	let count = $state(0);
	let doubled = $derived(count * 2);
</script>

<button onclick={() => count++}>{doubled}</button>
```

Recomputed when its dependencies change. The expression must be free of side
effects. For anything that needs statements, use **`$derived.by`**:

```js
let numbers = $state([1, 2, 3]);
let total = $derived.by(() => {
	let sum = 0;
	for (const n of numbers) sum += n;
	return sum;
});
```

**Overriding.** A derived value can be reassigned temporarily — useful for
optimistic UI — and recomputes when its dependencies next change:

```svelte
<script>
	let { post, like } = $props();
	let likes = $derived(post.likes);

	async function onclick() {
		likes += 1;
		try {
			await like();
		} catch {
			likes -= 1;
		}
	}
</script>
```

Reactivity is push-pull: a change is signalled at once, but a derived value is
only recomputed when it is next read.

## `$effect`

Runs after the DOM updates, re-running whenever state it read changes:

```svelte
<script>
	let size = $state(50);
	let color = $state('#ff3e00');
	let canvas;

	$effect(() => {
		const context = canvas.getContext('2d');
		context.clearRect(0, 0, canvas.width, canvas.height);
		context.fillStyle = color;
		context.fillRect(0, 0, size, size);
	});
</script>

<canvas bind:this={canvas} width="100" height="100"></canvas>
```

**Teardown.** Return a function to clean up before the next run and on
unmount:

```js
$effect(() => {
	const interval = setInterval(() => (count += 1), milliseconds);
	return () => clearInterval(interval);
});
```

- **`$effect.pre`** — runs before the DOM updates (e.g. to preserve a scroll
  position).
- **`$effect.root`** — a manually controlled effect scope outside component
  initialisation; returns a function that destroys it.
- **`$effect.tracking`** — whether the code is running inside a tracking
  context (an effect or the template).
- **`$effect.pending`** — how many promises are pending in the current
  boundary.

## `$props`

```svelte
<!-- Parent.svelte -->
<MyComponent adjective="cool" />

<!-- MyComponent.svelte -->
<script>
	let { adjective = 'happy', ...others } = $props();
</script>

<p>this component is {adjective}</p>
```

Destructuring gives fallback values, renaming and rest props. Typed in
TypeScript:

```svelte
<script lang="ts">
	let { adjective }: { adjective: string } = $props();
</script>
```

**`$props.id()`** — an id unique to the component instance and consistent
between server and client rendering:

```svelte
<script>
	const uid = $props.id();
</script>

<label for="{uid}-name">Name</label>
<input id="{uid}-name" type="text" />
```

Do not mutate props: pass a callback prop, or make the prop bindable.

## `$bindable`

```svelte
<!-- FancyInput.svelte -->
<script>
	let { value = $bindable('fallback'), ...props } = $props();
</script>

<input bind:value {...props} />

<!-- Parent.svelte -->
<FancyInput bind:value={message} />
```

Use sparingly: two-way binding makes data flow harder to follow.

## Practice

1. `$state` for values that change; `$state.raw` for large values replaced
   wholesale.
2. `$derived` for anything computed from state — not an `$effect` that writes
   state.
3. `$effect` only for real side effects (DOM, network, timers), with teardown.
4. Never mutate props; use callbacks or `$bindable`.
5. Type props in TypeScript.

## From Svelte 4

| Svelte 4 | Svelte 5 |
|---|---|
| `let count = 0;` (top-level `let` was reactive) | `let count = $state(0);` |
| `$: doubled = count * 2;` | `let doubled = $derived(count * 2);` |
| `$: console.log(count);` | `$effect(() => console.log(count));` |
| `export let adjective = 'happy';` | `let { adjective = 'happy' } = $props();` |
