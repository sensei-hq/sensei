// Type shim for @rokkit/states, which ships untyped JS source (its package
// `types` field points at an unpublished dist/). Discovered as the
// `@types/rokkit__states` package via tsconfig `typeRoots`. Each export is
// declared as both a type and a value (= any) so it works in either position
// (@rokkit/ui uses e.g. `ProxyItem` as a type). Upstream fix tracked in
// docs/backlog.md (rokkit type gaps).
declare module '@rokkit/states' {
  export type ProxyItem = any;
  export const ProxyItem: any;
  export type ProxyTree = any;
  export const ProxyTree: any;
  export type ProxyTable = any;
  export const ProxyTable: any;
  export type ProxyTableTree = any;
  export const ProxyTableTree: any;
  export type Wrapper = any;
  export const Wrapper: any;
  export type LazyWrapper = any;
  export const LazyWrapper: any;
  export type LazyProxyItem = any;
  export const LazyProxyItem: any;
  export const messages: any;
  export const commands: any;
  export const alerts: any;
  export const vibe: any;
}
