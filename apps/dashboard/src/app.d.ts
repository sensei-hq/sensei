import type { AuthSession } from 'kavach';

declare global {
  namespace App {
    interface Locals {
      session: AuthSession | null;
    }
  }
}

export {};
