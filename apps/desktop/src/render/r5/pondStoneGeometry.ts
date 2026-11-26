import type { PondBasinSnapshot, PondStone, PondStonePreset, PondWoodPreset } from '../../world';
import art from '../assets/r5/manifest.json';
import slopeProfile from '../assets/r5/pond-composed/slope-profile.json';
import bedGrit from '../assets/r5/pond/bed-grit.png';
import mainRock from '../assets/r5/pond/rock.png';
import mainShadow from '../assets/r5/pond/rock-shadow.png';
import pebble from '../assets/r5/pond/pebble.png';
import pebbleShadow from '../assets/r5/pond/pebble-shadow.png';
import bedBiofilm from '../assets/r5/pond/bed-biofilm.png';
import landscapeBark from '../assets/r5/pond-landscape/pond-wood.png';
import landscapeStone from '../assets/r5/pond-landscape/pond-stone.png';
import landscapeMoss from '../assets/r5/pond-landscape/pond-moss.png';
import { type Point, clamp } from './geometry';
import { hitSprite } from './sceneArt';
import { pondStructureProfile, type PondProfileBand } from './pondStructureProfiles';

export const bedY = 242;
export const bedLeft = 104;
export const bedWidth = 288;

const { bottomStones, bottomWoods, objects: { rock } } = art.pond;
const presets = { main: { ...rock, ...bottomStones.main }, pebble: bottomStones.pebble };
const urls = { bedGrit, mainRock, mainShadow, pebble, pebbleShadow } as const;
const woodUrls = import.meta.glob<string>('../assets/r5/pond/bottom-wood-*.png', { eager: true, query: '?url', import: 'default' });
const woodParts = ['back', 'front', 'growth', 'shadow'] as const;
type WoodPart = typeof woodParts[number];
type FlatWoodPreset = 'branch' | 'root';
type WoodArt = Record<FlatWoodPreset, Record<WoodPart, HTMLImageElement>> & { bedBiofilm: HTMLImageElement };
export type PondStoneArt = Record<keyof typeof urls, HTMLImageElement> & { wood?: WoodArt };
export type PondTerrainShape = { kind: 'stone'; preset: PondStonePreset } | { kind: 'wood'; preset: PondWoodPreset };
export type PondTerrainObject = PondTerrainShape & { x: number; id: number };
export type PondStoneDraft = PondTerrainShape & { x: number; id: number | null };
export type PondStoneHint = 'out-of-ground' | 'occupied' | 'ground';

type TerrainArtCache = { stones?: Promise<PondStoneArt>; wood?: Promise<WoodArt> };
const originalArt: TerrainArtCache = {};
const landscapeArt: TerrainArtCache = {};

type MaterialSample = { x: number; y: number; width: number; height: number; rise?: number };
const stoneSamples = {
  main: { x: 18, y: 8, width: 268, height: 168 },
  pebble: { x: 70, y: 62, width: 164, height: 100 },
};
// Sample along the grain of the diagonal reference branch. The active wood remains low and horizontal.
const woodSamples: Record<FlatWoodPreset, MaterialSample> = {
  branch: { x: 155, y: 370, width: 1370, height: 63, rise: -272 },
  root: { x: 444, y: 285, width: 763, height: 45, rise: -136 },
};
const mossSample: MaterialSample = { x: 84, y: 129, width: 230, height: 19, rise: -90 };

async function decodeImage(url: string) {
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

async function decodeWood(name: FlatWoodPreset) {
  const [back, front, growth, shadow] = await Promise.all(woodParts.map(async (key) => {
    const url = woodUrls[`../assets/r5/${bottomWoods[name].sprites[key]}`];
    if (!url) throw new Error(`Missing pond wood art: ${name}/${key}`);
    return decodeImage(url);
  }));
  return { back, front, growth, shadow };
}

async function materialFinish(
  original: HTMLImageElement, source: HTMLImageElement, sample: MaterialSample, strength: number,
  textureY = 0, textureHeight = original.naturalHeight,
) {
  const { naturalWidth: width, naturalHeight: height } = original;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('Unable to compose pond terrain materials');

  const scaleX = width / sample.width;
  const scaleY = textureHeight / sample.height;
  const slope = (sample.rise ?? 0) / sample.width;
  context.imageSmoothingQuality = 'high';
  context.setTransform(scaleX, -scaleY * slope, 0, scaleY, -scaleX * sample.x,
    textureY + scaleY * (slope * sample.x - sample.y));
  context.drawImage(source, 0, 0);
  const texture = context.getImageData(0, 0, width, height).data;

  context.resetTransform();
  context.clearRect(0, 0, width, height);
  context.drawImage(original, 0, 0);
  const layer = context.getImageData(0, 0, width, height);
  for (let pixel = 0; pixel < layer.data.length; pixel += 4) {
    if (layer.data[pixel + 3] === 0) continue;
    const blend = strength * texture[pixel + 3] / 255;
    for (let channel = pixel; channel < pixel + 3; channel++) {
      layer.data[channel] += (texture[channel] - layer.data[channel]) * blend;
    }
  }
  // Passes and picking use the same original coverage, including partial alpha at all contact edges.
  context.putImageData(layer, 0, 0);
  return decodeImage(canvas.toDataURL());
}

async function finishStones(stones: PondStoneArt) {
  const stone = await decodeImage(landscapeStone);
  const [mainRock, pebble] = await Promise.all([
    materialFinish(stones.mainRock, stone, stoneSamples.main, .84),
    materialFinish(stones.pebble, stone, stoneSamples.pebble, .78),
  ]);
  return { ...stones, mainRock, pebble };
}

async function finishWood(wood: WoodArt) {
  const [bark, moss] = await Promise.all([decodeImage(landscapeBark), decodeImage(landscapeMoss)]);
  async function finishPreset(preset: FlatWoodPreset) {
    const { upperAlphaY: top, anchor } = bottomWoods[preset];
    const original = wood[preset];
    const thickness = anchor[1] - top + 6;
    const [back, front, growth] = await Promise.all([
      materialFinish(original.back, bark, woodSamples[preset], .79, top, thickness),
      materialFinish(original.front, bark, woodSamples[preset], .79, top, thickness),
      materialFinish(original.growth, moss, mossSample, .59, top - 3, 14),
    ]);
    return { ...original, back, front, growth };
  }
  const [branch, root] = await Promise.all([finishPreset('branch'), finishPreset('root')]);
  return { ...wood, branch, root };
}

function loadOriginalStones() {
  originalArt.stones ??= Promise.all([urls.bedGrit, urls.mainRock, urls.mainShadow, urls.pebble, urls.pebbleShadow].map(decodeImage))
    .then(([bedGrit, mainRock, mainShadow, pebble, pebbleShadow]) => ({ bedGrit, mainRock, mainShadow, pebble, pebbleShadow }))
    .catch((error: unknown) => {
      originalArt.stones = undefined;
      throw error;
    });
  return originalArt.stones;
}

function loadOriginalWood() {
  originalArt.wood ??= Promise.all([decodeWood('branch'), decodeWood('root'), decodeImage(bedBiofilm)])
    .then(([branch, root, bedBiofilm]) => ({ branch, root, bedBiofilm })).catch((error: unknown) => {
      originalArt.wood = undefined;
      throw error;
    });
  return originalArt.wood;
}

export function loadPondStoneArt(includeWood = false, landscape = false) {
  const stones = loadOriginalStones();
  if (!landscape) {
    return includeWood ? Promise.all([stones, loadOriginalWood()]).then(([stones, wood]) => ({ ...stones, wood })) : stones;
  }
  landscapeArt.stones ??= stones.then(finishStones).catch((error: unknown) => {
    landscapeArt.stones = undefined;
    throw error;
  });
  if (!includeWood) return landscapeArt.stones;
  landscapeArt.wood ??= loadOriginalWood().then(finishWood).catch((error: unknown) => {
    landscapeArt.wood = undefined;
    throw error;
  });
  return Promise.all([landscapeArt.stones, landscapeArt.wood]).then(([stones, wood]) => ({ ...stones, wood }));
}

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

function spriteScale(shape: NonNullable<ReturnType<typeof geometry>>) {
  return shape.heightPx / (shape.anchor[1] - shape.upperAlphaY);
}

type TerrainPass = 'shadow' | 'body' | 'mask' | 'behind' | 'front';

function terrainSprites(art: PondStoneArt, item: PondTerrainShape, pass: TerrainPass) {
  if (item.kind === 'wood') {
    if (item.preset !== 'branch' && item.preset !== 'root') return [];
    const wood = art.wood?.[item.preset];
    if (!wood) return [];
    if (pass === 'shadow' || pass === 'front') return [wood[pass]];
    if (pass === 'behind') return [wood.back, wood.growth];
    return pass === 'mask' ? [wood.back, wood.front] : [wood.back, wood.growth, wood.front];
  }
  if (item.preset === 'flatStoneCave') return [];
  const shadow = pass === 'shadow';
  return [item.preset === 'main' ? art[shadow ? 'mainShadow' : 'mainRock'] : art[shadow ? 'pebbleShadow' : 'pebble']];
}

export function drawPondTerrain(
  context: CanvasRenderingContext2D, art: PondStoneArt, item: PondTerrainShape & { x: number },
  pass: TerrainPass = 'body',
) {
  const shape = geometry(item);
  if (!shape) return;
  const { x, y } = bedPoint(item.x);
  const scale = spriteScale(shape);
  for (const image of terrainSprites(art, item, pass)) {
    context.drawImage(image, x - shape.anchor[0], y - shape.anchor[1] * scale, shape.width, shape.height * scale);
  }
}

export function pickPondTerrain(objects: readonly PondTerrainObject[], at: Point, art: PondStoneArt) {
  for (let index = objects.length - 1; index >= 0; index--) {
    const item = objects[index];
    const shape = geometry(item);
    if (!shape) continue;
    const localX = Math.floor(at.x - bedPoint(item.x).x + shape.anchor[0]);
    const localY = Math.floor((at.y - bedY) / spriteScale(shape) + shape.anchor[1]);
    if (localX >= 0 && localX < shape.width && localY >= 0 && localY < shape.height
      && terrainSprites(art, item, 'mask').some((image) => hitSprite(image, localX, localY))) return item;
  }
}

export function markPondStone(context: CanvasRenderingContext2D, item: PondTerrainShape & { x: number }, hint?: PondStoneHint) {
  const shape = geometry(item);
  const { x, y } = bedPoint(item.x);
  context.save();
  context.strokeStyle = hint === undefined ? '#b6eddb' : hint === 'ground' ? '#d3e9ea' : '#ffd39d';
  if (hint) context.setLineDash([3, 2]);
  if (shape) {
    const { halfSpanPx, heightPx } = shape;
    context.strokeRect(x - halfSpanPx - 2.5, y - heightPx - 2.5, (halfSpanPx + 2) * 2, heightPx + 6);
  } else {
    for (const section of [slopeProfile.ridge, slopeProfile.foot]) {
      context.beginPath();
      context.moveTo(x + section[0][0], y - section[0][2]);
      for (const [dx, , top] of section.slice(1)) context.lineTo(x + dx, y - top);
      for (const [dx, bottom] of section.slice().reverse()) context.lineTo(x + dx, y - bottom);
      context.closePath();
      context.stroke();
    }
  }
  context.restore();
}
