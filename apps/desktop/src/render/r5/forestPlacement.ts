import type { BarkShelterPreset, ForestFernPatch, ForestMossPatch, ForestRock, ForestTerrainKind, LeafPatchPreset, StoneworkPreset, WorldSnapshot } from '../../world';
import { clamp, type Point } from './geometry';
import geometry from '../assets/r5/placement-geometry.json';

export type ForestShape = { kind: ForestTerrainKind; preset?: StoneworkPreset | BarkShelterPreset | LeafPatchPreset };
export type ForestDraft = ForestShape & { id: number | null; position: Point; mirrored: boolean };
export type ForestObject = ForestShape & ForestRock & {
  footCells?: readonly number[]; passageCells?: readonly number[]; occupantIds?: readonly string[];
  initialMgCnp?: readonly number[]; remainingMgCnp?: readonly number[]; cellIndex?: number;
};

// The legacy snapshot exposes a single rock position without a terrain object ID.
export const legacyForestRockId = 0;

export function forestObjects(snapshot: WorldSnapshot): ForestObject[] {
  const layout = snapshot.forestRocks;
  if (!layout) return [{ kind: 'rock', id: legacyForestRockId, position: snapshot.rock, mirrored: false }];
  const entries: ForestObject[] = [
    ...layout.rocks.map((rock): ForestObject => ({ ...rock, kind: 'rock' })),
    ...(layout.logs ?? []).map((log): ForestObject => ({ ...log, kind: 'log' })),
    ...(layout.stonework ?? []).map((shape): ForestObject => ({ ...shape, kind: 'stonework' })),
    ...(layout.barkShelters ?? []).map((shelter): ForestObject => ({ ...shelter, kind: 'barkShelter' })),
    ...(layout.leafPatches ?? []).map((patch): ForestObject => ({ ...patch, kind: 'leafPatch' })),
  ];
  const groundLayer = (entry: ForestObject) => entry.kind === 'leafPatch' ? 0 : 1;
  return entries.sort((a, b) => groundLayer(a) - groundLayer(b) || a.position.y - b.position.y || a.id - b.id);
}

export const forestPlacementGeometry = geometry.forest;
const { stonework, barkShelters, leafPatches, logLengthMm, referenceWidthMm } = forestPlacementGeometry;

function stoneworkArt(preset: ForestShape['preset']) {
  return stonework[preset === 'pair' || preset === 'cluster' ? preset : 'slab'];
}

function barkShelterArt(preset: ForestShape['preset']) {
  if (preset === 'split' || preset === 'rootArch' || preset === 'branchBridge') return barkShelters[preset];
  return barkShelters.arch;
}

function leafPatchArt(preset: ForestShape['preset']) {
  return leafPatches[preset === 'heap' || preset === 'fragments' ? preset : 'leaves'];
}

export function leafPatchFrame(item: Pick<ForestObject, 'initialMgCnp' | 'remainingMgCnp'>) {
  const { initialMgCnp, remainingMgCnp } = item;
  if (!initialMgCnp || !remainingMgCnp) return 'default';
  if (remainingMgCnp.every((stock) => stock <= 0)) return 'depleted';

  // Carbon stock selects an authored cel. The cel is never used to infer mass.
  const ratio = initialMgCnp[0] > 0 ? remainingMgCnp[0] / initialMgCnp[0] : 0;
  if (ratio >= .72) return 'default';
  if (ratio >= .36) return 'worn';
  return 'trace';
}

// Logical soil form documented by eco-core. This only indicates the latest displayed snapshot.
const cols = 12;
const rows = 4;
const minX = .04;
const minY = .58;
const maxX = .96;
const maxY = .94;
const cellWidth = (maxX - minX) / cols;
const cellHeight = (maxY - minY) / rows;

function cellIndex({ x, y }: Point) {
  const col = clamp(Math.floor((x - minX) / cellWidth), 0, cols - 1);
  const row = clamp(Math.floor((y - minY) / cellHeight), 0, rows - 1);
  return row * cols + col;
}

export const forestSoilColumns = cols;
export const forestSoilRows = rows;

export function forestSoilCell(index: number) {
  return {
    index,
    x: minX + index % cols * cellWidth,
    y: minY + Math.floor(index / cols) * cellHeight,
    width: cellWidth,
    height: cellHeight,
  };
}

export function forestSoilIndex({ x, y }: Point) {
  return x >= minX && x <= maxX && y >= minY && y <= maxY ? cellIndex({ x, y }) : undefined;
}

function stoneworkCells(shape: ForestDraft) {
  const { position, mirrored, preset } = shape;
  const sign = mirrored ? -1 : 1;
  return stoneworkArt(preset).parts.map(({ offsetMm }) => cellIndex({
    x: position.x + sign * offsetMm / referenceWidthMm, y: position.y,
  })).sort((a, b) => a - b);
}

function structureCells(object: ForestObject) {
  if (object.kind === 'barkShelter') return [...(object.footCells ?? []), ...(object.passageCells ?? [])];
  if (object.kind === 'leafPatch') return [object.cellIndex ?? cellIndex(object.position)];
  if (object.kind === 'stonework') return stoneworkCells(object);
  if (object.kind === 'rock') return [cellIndex(object.position)];
  const half = logLengthMm / referenceWidthMm / 2;
  const left = cellIndex({ ...object.position, x: object.position.x - half });
  const right = cellIndex({ ...object.position, x: object.position.x + half });
  return Array.from({ length: right - left + 1 }, (_, index) => left + index);
}

export function forestStockSite(point: Point, objects: readonly ForestObject[]) {
  const cell = forestSoilIndex(point);
  if (cell === undefined) return 'outside';
  const blocked = objects.some((object) => object.kind !== 'leafPatch'
    && (object.kind === 'barkShelter' ? object.footCells ?? [] : structureCells(object)).includes(cell));
  return blocked ? 'blocked' : 'clear';
}

export function forestMossSite(point: Point, objects: readonly ForestObject[], patches: readonly ForestMossPatch[], moving?: number) {
  const ground = forestStockSite(point, objects);
  if (ground !== 'clear') return ground;
  return patches.some((patch) => patch.id !== moving && patch.cellIndex === cellIndex(point)) ? 'blocked' : 'clear';
}

export function forestFernSite(point: Point, objects: readonly ForestObject[], patches: readonly ForestFernPatch[], moving?: number) {
  const cell = forestSoilIndex(point);
  if (cell === undefined) return 'outside';
  const blocked = objects.some((object) => object.kind !== 'leafPatch' && structureCells(object).includes(cell))
    || patches.some((patch) => patch.id !== moving && patch.cellIndex === cell);
  return blocked ? 'blocked' : 'clear';
}

export type PlacementHint = {
  site: 'ground' | 'out-of-ground' | 'occupied';
  cells: { index: number; occupied: boolean; role: 'solid' | 'passage' | 'substrate'; x: number; y: number; width: number; height: number }[];
};

export function forestPlacementHint(draft: ForestDraft, objects: readonly ForestObject[], animals: readonly Point[],
  ferns: readonly ForestFernPatch[] = []): PlacementHint {
  const { kind, preset, id, position: { x, y } } = draft;
  const bark = kind === 'barkShelter' ? barkShelterArt(preset) : undefined;
  const leaves = kind === 'leafPatch' ? leafPatchArt(preset) : undefined;
  const [left, right] = bark?.placementRange ?? leaves?.placementRange ?? (kind === 'stonework' ? stoneworkArt(preset).placementRange
    : kind === 'log' ? [.33, .67] : [.085, .915]);
  const [low, high] = bark ? [.70, .81] : [.60, .91];
  if (!(x >= left && x <= right && y >= low && y <= high)) return { site: 'out-of-ground', cells: [] };
  if (kind !== 'stonework' && !bark && !leaves) {
    const footprint = structureCells({ ...draft, id: id ?? 0 });
    const overlapsLeaves = objects.some((item) => item.kind === 'leafPatch'
      && footprint.includes(item.cellIndex ?? cellIndex(item.position)));
    const overlapsFerns = ferns.some(({ cellIndex }) => footprint.includes(cellIndex));
    return { site: overlapsLeaves || overlapsFerns ? 'occupied' : 'ground', cells: [] };
  }

  const occupied = new Set<number>();
  const animalCells = new Set(animals.map(cellIndex));
  for (const object of objects) {
    if (object.id !== id) for (const index of structureCells(object)) occupied.add(index);
  }
  if (!leaves) for (const fern of ferns) occupied.add(fern.cellIndex);
  const pointForCell = (index: number, role: PlacementHint['cells'][number]['role']) => ({
    ...forestSoilCell(index),
    occupied: occupied.has(index) || (role === 'solid' && animalCells.has(index)),
    role,
  });
  const cells = leaves ? [pointForCell(cellIndex({ x, y }), 'substrate')] : bark ? (() => {
    const offset = bark.footOffsetMm / referenceWidthMm;
    const center = cellIndex({ x, y });
    return [
      pointForCell(cellIndex({ x: x - offset, y }), 'solid'),
      pointForCell(cellIndex({ x: x + offset, y }), 'solid'),
      ...[center - cols, center, center + cols].map((index) => pointForCell(index, 'passage')),
    ];
  })() : stoneworkCells(draft).map((index) => pointForCell(index, 'solid'));
  return { site: cells.some((cell) => cell.occupied) ? 'occupied' : 'ground', cells };
}
