// Type declarations for kavach packages and virtual modules.

declare module '$kavach/auth' {
  import type { Handle } from '@sveltejs/kit';
  export const kavach: {
    handle: Handle;
  };
  export const adapter: unknown;
  export const logger: unknown;
}

declare module '@kavach/vite' {
  import type { Plugin } from 'vite';
  export function kavach(options?: Record<string, unknown>): Plugin;
}

declare module '@kavach/adapter-supabase' {
  export interface ActionResponse {
    data: unknown | null;
    error: unknown | null;
    status: number;
    count?: number;
  }

  export function getActions(
    client: unknown,
    schema?: string
  ): {
    get(entity: string, params?: Record<string, unknown>): Promise<ActionResponse>;
    put(entity: string, data: Record<string, unknown>): Promise<ActionResponse>;
    post(entity: string, data: Record<string, unknown>): Promise<ActionResponse>;
    patch(entity: string, input?: { data?: Record<string, unknown>; filter?: Record<string, string> }): Promise<ActionResponse>;
    delete(entity: string, input?: { filter?: Record<string, string> }): Promise<ActionResponse>;
    call(entity: string, data: Record<string, unknown>): Promise<ActionResponse>;
    connection: unknown;
  };
}

declare module '@kavach/query' {
  export interface FilterDescriptor {
    column: string;
    op: string;
    value: string | string[] | null;
  }

  export interface OrderDescriptor {
    column: string;
    ascending: boolean;
  }

  export interface QueryParams {
    columns: string;
    filters: FilterDescriptor[];
    orders: OrderDescriptor[];
    limit?: number;
    offset?: number;
    count?: string;
  }

  export function parseFilter(filter?: Record<string, string>): FilterDescriptor[];
  export function parseOrder(order?: string): OrderDescriptor[];
  export function parseQueryParams(data?: Record<string, unknown>): QueryParams;
}
