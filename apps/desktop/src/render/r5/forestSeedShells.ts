import definition from '../assets/r5/forest-composed/definition.json';
import type { Point } from './geometry';

export const seedShellGroups = definition.seedShellGroups;
export type SeedShellDensity = keyof typeof seedShellGroups;

export function seedShellGroup(density?: string) {
  return density && Object.hasOwn(seedShellGroups, density) ? seedShellGroups[density as SeedShellDensity] : undefined;
}

export function isSeedShellSite({ spanMm }: { spanMm: number }, { x, y }: Point) {
  const margin = spanMm / (2 * definition.seedShellReferenceWidthMm);
  const [top, bottom] = definition.seedShellPlacementY;
  return x >= margin && x <= 1 - margin && y >= top && y <= bottom;
}
