/**
 * Type shims for @rokkit/* packages whose published TS source is walked by
 * svelte-check but is missing companion declaration files.
 *
 * Background
 * ----------
 * @rokkit packages ship `.ts` and `.js` source directly (no compiled `.d.ts`
 * for `.js` modules). svelte-check follows our app's imports into the
 * package source and either (a) infers implicit `any` for `.js` files, or
 * (b) walks `.svelte` files that don't have generated declarations.
 *
 * Strategy
 * --------
 * Declare each problem module here with the surface types actually used.
 * We export named symbols as classes/values so downstream `.ts` files in
 * @rokkit/ui that import `ProxyItem` as a type continue to type-check.
 * Internal members of @rokkit/* aren't typed — we only need the boundary
 * to compile.
 *
 * Tracked upstream at https://github.com/jerrythomas/rokkit. Re-evaluate
 * when @rokkit ships proper `.d.ts` bundles alongside `.js` source.
 */

declare module '@rokkit/states' {
  // Surface exports observed in @rokkit/states/src/index.js. Bodies are
  // intentionally untyped — we only need name resolution + usability as
  // a type, not full type safety from inside the package.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export class ProxyItem<T = unknown> { constructor(...args: any[]); [key: string]: any; }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export class LazyProxyItem<T = unknown> { constructor(...args: any[]); [key: string]: any; }
  export const BASE_FIELDS: Record<string, string>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const alerts: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export class TableController { constructor(...args: any[]); [key: string]: any; }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const vibe: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export class ListController { constructor(...args: any[]); [key: string]: any; }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const messages: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export class ProxyTree { constructor(...args: any[]); [key: string]: any; }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export class Wrapper { constructor(...args: any[]); [key: string]: any; }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export class LazyWrapper { constructor(...args: any[]); [key: string]: any; }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export function watchMedia(...args: any[]): any;
  export const defaultBreakpoints: Record<string, number>;
}
