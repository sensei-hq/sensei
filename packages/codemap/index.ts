// @sensei/codemap — the Code map (M17), shared by the desktop app and the dōjō.
export { default as CodeMap } from './CodeMap.svelte';
export { CodeMapState, ROLES } from './state.svelte';
export type { Level, Lens, Crumb } from './state.svelte';
export { layoutLevel0, groupOrder } from './layout';
export { layoutWheel } from './wheel';
export { matrix, cycleBands, cycleOf, zonesView, couplingBands, complexityShare } from './lenses';
export { inspect, LIBRARIES } from './inspector';
export type { Inspection, InspectInput, Tone } from './inspector';
export { listView } from './list';
export type { ListView, ListRow, Badge } from './list';
export { visibleBands, bandTone, legendFor, complexityTier, shownFiles, searchModules, MIN_WEIGHTS } from './view';
export type { EdgeMode, BandFilter, LegendEntry, ComplexityTier } from './view';
export type {
  GroupBy, Direction, Role, RoleCounts, Block, Band, Level0, ModuleFile, Column, Neighbour, Level1, SymbolRef, Level2, Read,
  CodeMapPort, ZonePoint, Zones, Coupling, CoChange,
} from './types';
