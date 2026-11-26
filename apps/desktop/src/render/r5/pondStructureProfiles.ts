import profiles from '../assets/r5/pond-composed/structure-profiles.json';
import type { PondStonePreset, PondWoodPreset } from '../../world';

export type PondStructurePreset = keyof typeof profiles;
export type PondProfileBand = { left: readonly number[]; right: readonly number[] };

const structures = new Map(Object.entries(profiles).map(([variant, { anchorRange, bands, swimClearance }]) => {
  const edges = bands.flat();
  return [variant, {
    variant: variant as PondStructurePreset, anchorRange, swimClearance,
    bands: bands.map(([left, right]): PondProfileBand => ({ left, right })),
    left: Math.min(...edges.map(([x]) => x)), right: Math.max(...edges.map(([x]) => x)),
    bottom: Math.min(...edges.map(([, bottom]) => bottom)), top: Math.max(...edges.map(([, , top]) => top)),
  }] as const;
}));

export function pondStructureProfile(preset: PondStonePreset | PondWoodPreset) {
  return structures.get(preset);
}
