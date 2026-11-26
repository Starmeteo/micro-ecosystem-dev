import type { PondBasinSnapshot, PondRibbonPlantPreset, PondWaterPlantPatch } from '../../world';
import { clamp } from './geometry';
import { bedWidth, pondBedContactBlocked, type PondStoneHint } from './pondStoneGeometry';

export const waterPlantDoseMgC = 50;
export const waterPlantLimit = 8;
export const waterPlantRange = [.08, .92] as const;
export const waterPlantSpacingPx = 14;

export const ribbonPlantDimensions: Record<PondRibbonPlantPreset, { leafArcMm: number; leafWidthMm: number; spreadMm: number }> = {
  small: { leafArcMm: 120, leafWidthMm: 3, spreadMm: 40 },
  medium: { leafArcMm: 180, leafWidthMm: 4.5, spreadMm: 70 },
  large: { leafArcMm: 240, leafWidthMm: 6, spreadMm: 100 },
};

export type PondWaterPlantDraft = { id: number | null; x: number; ribbonPreset?: PondRibbonPlantPreset };

export function waterPlantGrowth(patch: PondWaterPlantPatch) {
  return clamp(patch.stockMgCnp[0] / patch.initialMgCnp[0], 0, 2);
}

export function waterPlantFrame(patch: PondWaterPlantPatch) {
  const growth = waterPlantGrowth(patch);
  if (growth === 0) return 'depleted';
  if (!patch.submerged) return 'exposed';
  if (growth < .45) return 'sparse';
  return growth < .95 ? 'growing' : 'full';
}

export function waterPlantSite({ id, x }: PondWaterPlantDraft, pond: PondBasinSnapshot): PondStoneHint {
  const patches = pond.waterPlantPatches;
  if (!patches || (id === null && !pond.wet) || !(x >= waterPlantRange[0] && x <= waterPlantRange[1])) return 'out-of-ground';
  return (id === null && patches.length >= waterPlantLimit) || pondBedContactBlocked(x, pond)
    || patches.some((patch) => patch.id !== id && Math.abs(patch.x - x) * bedWidth < waterPlantSpacingPx) ? 'occupied' : 'ground';
}
