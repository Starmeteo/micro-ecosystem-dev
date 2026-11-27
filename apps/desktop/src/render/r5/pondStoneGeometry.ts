import type { PondBasinSnapshot, PondStone, PondStonePreset, PondWoodPreset } from '../../world';
import art from '../assets/r5/placement-geometry.json';
import slopeProfile from '../assets/r5/pond-composed/slope-profile.json';
import { type Point, clamp } from './geometry';
import { pondStructureProfile, type PondProfileBand } from './pondStructureProfiles';

export const bedY = 242;
export const bedLeft = 104;
export const bedWidth = 288;

const { bottomStones: presets, bottomWoods } = art.pond;
export type PondTerrainShape = { kind: 'stone'; preset: PondStonePreset } | { kind: 'wood'; preset: PondWoodPreset };
export type PondTerrainObject = PondTerrainShape & { x: number; id: number };
export type PondStoneDraft = PondTerrainShape & { x: number; id: number | null };
export type PondStoneHint = 'out-of-ground' | 'occupied' | 'ground';

export function stoneWaterHeight(pond: PondBasinSnapshot) {
  return pond.wet ? Math.max(1, 208 * clamp(pond.depthCm / 12, 0, 1)) : 0;
}

export function bedPoint(x: number): Point {
  return { x: bedLeft + bedWidth * x, y: bedY };
}

export function pondRockShrimpPoint(shrimp: Point, pond: PondBasinSnapshot): Point {
  return { x: bedPoint(shrimp.x).x, y: bedY - (1 - shrimp.y) * stoneWaterHeight(pond) };
}

export function pondTerrainObjects(pond: PondBasinSnapshot): PondTerrainObject[] {
  return [
    ...(pond.stones ?? []).map((stone): PondTerrainObject => ({ ...stone, kind: 'stone' })),
    ...(pond.woods ?? []).map((wood): PondTerrainObject => ({ ...wood, kind: 'wood' })),
  ];
}

function geometry(shape: PondTerrainShape) {
  if (shape.kind === 'stone') return shape.preset === 'flatStoneCave' ? undefined : presets[shape.preset];
  return shape.preset === 'branch' || shape.preset === 'root' ? bottomWoods[shape.preset] : undefined;
}

type ProfileBand = PondProfileBand;
const slopeBands: ProfileBand[] = [
  ...slopeProfile.ridge.slice(1).map((right, index) => ({ left: slopeProfile.ridge[index], right })),
  { left: slopeProfile.foot[0], right: slopeProfile.foot[1] },
];

function bandHeight({ left, right }: ProfileBand, x: number, edge: 1 | 2) {
  return left[edge] + (x - left[0]) / (right[0] - left[0]) * (right[edge] - left[edge]);
}

export function pondMossSupport(item: PondTerrainShape, offset = 0) {
  if (!Number.isFinite(offset) || Math.abs(offset) > 1) return;
  if (pondStructureProfile(item.preset)) return;
  if (item.kind === 'stone') {
    if (item.preset === 'pebble') return;
    return { height: presets.main.heightPx, halfSpan: 10, center: 0 };
  }
  if (item.preset === 'slope') {
    const { supportCenter: center, supportHalfSpan: halfSpan } = slopeProfile;
    const x = center + offset * halfSpan;
    const band = slopeBands.find(({ left, right }) => left[0] <= x && x <= right[0]);
    if (band) return { height: bandHeight(band, x, 2), halfSpan, center };
    return;
  }
  if (item.preset !== 'branch' && item.preset !== 'root') return;
  const { heightPx: height, surfaceHalfSpanPx: halfSpan } = bottomWoods[item.preset];
  return { height, halfSpan, center: 0 };
}

const waterGap = 4;
const shrimpHalfLength = 8;

export function pondMainStoneRestPoint(stone: PondStone, offset: number, column: number): Point | undefined {
  if (stone.preset !== 'main' || !Number.isFinite(offset) || Math.abs(offset) > 1 || column <= 0) return;
  const x = stone.x + offset * (presets.main.halfSpanPx - shrimpHalfLength) / bedWidth;
  const y = 1 - (presets.main.heightPx + .25) / column;
  if (x >= 0 && x <= 1 && y >= .20 && y <= .96) return { x, y };
}

function bands(item: PondTerrainShape & { x: number }): ProfileBand[] {
  const x = item.x * bedWidth;
  const flat = geometry(item);
  if (flat) return [{ left: [x - flat.halfSpanPx, 0, flat.heightPx], right: [x + flat.halfSpanPx, 0, flat.heightPx] }];
  const profile = pondStructureProfile(item.preset);
  return (profile?.bands ?? slopeBands).map(({ left, right }) => ({
    left: [x + left[0], left[1], left[2]], right: [x + right[0], right[1], right[2]],
  }));
}

export function pondTerrainWettedWeight(item: PondTerrainObject, waterColumn: number) {
  // A side-profile weight for display. Horizontal bounds and the waterline clip each shaped band.
  let weight = 0;
  for (const band of bands(item)) {
    const left = Math.max(0, band.left[0]);
    const right = Math.min(bedWidth, band.right[0]);
    if (left >= right) continue;
    const breaks = [left, right];
    for (const edge of [1, 2] as const) {
      const rise = band.right[edge] - band.left[edge];
      if (rise === 0) continue;
      const crossing = band.left[0] + (waterColumn - band.left[edge]) / rise * (band.right[0] - band.left[0]);
      if (crossing > left && crossing < right) breaks.push(crossing);
    }
    breaks.sort((a, b) => a - b);
    for (let index = 1; index < breaks.length; index++) {
      const start = breaks[index - 1];
      const end = breaks[index];
      const midpoint = (start + end) / 2;
      const thickness = Math.min(waterColumn, bandHeight(band, midpoint, 2)) - bandHeight(band, midpoint, 1);
      weight += (end - start) * Math.max(0, thickness);
    }
  }
  return weight;
}

function profileBlocksRoot(profile: ProfileBand[], x: number) {
  const offset = x * bedWidth;
  return profile.some((band) => offset >= band.left[0]
    && offset <= band.right[0] && bandHeight(band, offset, 1) <= 0 && bandHeight(band, offset, 2) >= 0);
}

export function pondBedContactBlocked(x: number, pond: PondBasinSnapshot) {
  return pondTerrainObjects(pond).some((item) => profileBlocksRoot(bands(item), x));
}

export function pondStockSite(point: Point, pond: PondBasinSnapshot) {
  const cave = pond.stones?.some(({ preset }) => preset === 'flatStoneCave');
  const wide = Boolean(pond.woods?.length || cave);
  const minX = wide ? 0 : .06;
  const column = stoneWaterHeight(pond);
  const maxY = cave ? Math.max(.96, 1 - 1 / (column || 1)) : wide ? .96 : .92;
  if (!pond.wet || !(point.x >= minX && point.x <= 1 - minX && point.y >= .20 && point.y <= maxY)) return 'outside';
  const x = point.x * bedWidth;
  const height = (1 - point.y) * column;
  const blocked = pondTerrainObjects(pond).some((item) => {
    const flat = geometry(item);
    if (flat) return Math.abs(x - item.x * bedWidth) <= flat.halfSpanPx + shrimpHalfLength && height <= flat.heightPx;
    return shapedBlocksPoint(item, x, height);
  });
  return blocked ? 'blocked' : 'clear';
}

function bandsOverlap(a: ProfileBand, b: ProfileBand) {
  const left = Math.max(0, Math.max(a.left[0], b.left[0]) - waterGap / 2);
  const right = Math.min(bedWidth, Math.min(a.right[0], b.right[0]) + waterGap / 2);
  if (left > right) return false;
  let first = 0;
  let last = 1;
  for (const [above, below] of [[a, b], [b, a]]) {
    const clearance = (x: number) => bandHeight(above, x, 2) + waterGap - bandHeight(below, x, 1);
    const start = clearance(left);
    const end = clearance(right);
    if (start < 0 && end < 0) return false;
    if (start < 0) first = Math.max(first, start / (start - end));
    if (end < 0) last = Math.min(last, start / (start - end));
  }
  return first <= last;
}

export function stoneHint(draft: PondStoneDraft, pond: PondBasinSnapshot): PondStoneHint {
  const shape = geometry(draft);
  const [min, max] = shape?.placementRange ?? pondStructureProfile(draft.preset)?.anchorRange ?? slopeProfile.anchorRange;
  if (!Number.isFinite(draft.x) || draft.x < min || draft.x > max) return 'out-of-ground';
  const objects = pondTerrainObjects(pond);
  const count = draft.kind === 'wood' ? pond.woods?.length ?? 0 : pond.stones?.length ?? 0;
  if (draft.id === null && count >= (draft.kind === 'wood' ? 2 : 4)) return 'occupied';
  const profile = bands(draft);
  if (pond.waterPlantPatches?.some(({ x }) => profileBlocksRoot(profile, x))) return 'occupied';
  if (objects.some((item) => {
    if (item.id === draft.id && item.kind === draft.kind) return false;
    const flat = geometry(item);
    if (flat && shape) return Math.abs(item.x - draft.x) * bedWidth < flat.halfSpanPx + shape.halfSpanPx + 2 * shrimpHalfLength + waterGap;
    return bands(item).some((other) => profile.some((band) => bandsOverlap(band, other)));
  })) return 'occupied';
  const carried = new Map(draft.kind === 'wood' && draft.id !== null
    ? pond.shrimpAttachments?.filter(({ woodId }) => woodId === draft.id).map(({ shrimpId, offset }) => [shrimpId, offset]) : undefined);
  const depth = stoneWaterHeight(pond);
  if (pond.wet && pond.shrimp.some((shrimp) => {
    const offset = carried.get(shrimp.id);
    if (offset !== undefined) {
      const support = pondMossSupport(draft, offset);
      if (!support) return true;
      const x = draft.x + (support.center + offset * support.halfSpan) / bedWidth;
      const y = 1 - (support.height + .25) / depth;
      return x < 0 || x > 1 || y < .2 || y > .96;
    }
    const height = (1 - shrimp.y) * depth;
    if (shape) return Math.abs(shrimp.x - draft.x) * bedWidth <= shape.halfSpanPx + shrimpHalfLength && height <= shape.heightPx;
    const x = shrimp.x * bedWidth;
    return shapedBlocksPoint(draft, x, height);
  })) return 'occupied';
  return 'ground';
}

function shapedBlocksPoint(item: PondTerrainShape & { x: number }, x: number, height: number) {
  const [clearanceX, clearanceY] = pondStructureProfile(item.preset)?.swimClearance ?? [0, 0];
  return bands(item).some((band) => {
    if (x < band.left[0] - clearanceX || x > band.right[0] + clearanceX) return false;
    const testX = clamp(x, band.left[0], band.right[0]);
    return height >= bandHeight(band, testX, 1) - clearanceY && height <= bandHeight(band, testX, 2) + clearanceY;
  });
}
