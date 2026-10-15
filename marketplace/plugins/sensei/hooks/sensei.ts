// The sensei plugin's mods module: one register, one concern per file.
import type { Register } from 'claude-code'
import { registerRouting } from './routing.ts'
import { registerUsage } from './usage.ts'

export const register: Register = on => {
  registerUsage(on)
  registerRouting(on)
}
