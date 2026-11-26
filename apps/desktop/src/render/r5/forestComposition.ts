import type { FixtureAnimal, ForestScenery, WorldSnapshot } from '../../world';
import { clamp, type Point, type Rect } from './geometry';
import { type ForestDraft, type ForestObject, type PlacementHint, forestArt, forestSoilIndex, leafPatchFrame } from './forestPlacement';
import {
  type ComposedForestArt, type ForestPose, type ForestStyle, drawForestContact, drawForestSprite, forestHeight,
  forestHeroStyles, forestPixelsPerMm, forestPose, forestSceneryStyles, forestWidth, hitsForestSprite,
  markForestSprite, paintComposedForestBed, paintComposedForestLight,
} from './forestComposedArt';
import { paintForestSubstrate } from './forestSubstrate';
import { fernPart } from './forestFern';
import { paintForestSpringtails, type ForestSpringtailStock } from './forestSpringtails';
import { isSeedShellSite, seedShellGroup } from './forestSeedShells';
import { observeForestMycelium, paintForestMycelium, type ForestMyceliumObservation } from './forestMycelium';

export interface ForestLayerKey {
  kind: 'terrain' | 'scenery' | 'moss' | 'fern';
  id: number;
}

interface PaintedPart {
  style: ForestStyle;
  pose: ForestPose;
  heightSlice?: readonly [number, number];
}

export interface ForestLayer {
  key?: ForestLayerKey;
  parent?: ForestLayerKey | null;
  anchor: Point;
  parentMirror: number;
  mirror: number;
  depth: number;
  parts: PaintedPart[];
  terrain?: ForestObject;
  mycelium?: ForestMyceliumObservation;
  inside?: PaintedPart[];
  front?: PaintedPart[];
  draft?: boolean;
  invalidSite?: boolean;
}

export interface ForestSceneryDraft extends Point {
  id: number | null;
  style: string;
}

export interface ForestFernDraft {
  id: number | null;
  position: Point;
}

export function forestStage(point: Point): Point {
  return { x: point.x * forestWidth, y: point.y * forestHeight };
}

export function forestPosition(point: Point): Point {
  return { x: point.x / forestWidth, y: point.y / forestHeight };
}

export function composedForestCamera(zoom: number, point: Point): Rect {
  const width = forestWidth / zoom;
  const height = forestHeight / zoom;
  return {
    x: clamp(point.x - width / 2, 0, forestWidth - width),
    y: clamp(point.y - height / 2, 0, forestHeight - height),
    width, height,
  };
}

export function sameForestLayer(a?: ForestLayerKey, b?: ForestLayerKey) {
  return Boolean(a && b && a.kind === b.kind && a.id === b.id);
}

function physicalForestStyle(item: ForestObject, wetness: number): ForestStyle | undefined {
  if (item.kind === 'rock') return forestHeroStyles['rock-main'];
  if (item.kind === 'log') {
    return { type: 'ground-log', depth: 3, mm: 260, shade: .91, saturation: .85,
      shadow: [338, 39, .65, -17, -13], shadowAngle: -.18 };
  }
  if (item.kind === 'barkShelter') {
    if (item.preset === 'rootArch' || item.preset === 'branchBridge') return;
    const style = forestHeroStyles['log-main'];
    return item.preset === 'split' ? { ...style, mm: 154, offsetX: 17, flat: .87, depth: 3, contact: undefined,
      shadow: [184, 29, .60, -9, -13] } : style;
  }
  if (item.kind !== 'leafPatch') return;
  if (item.preset === 'fragments') {
    const stage = leafPatchFrame(item);
    if (stage === 'depleted') return;
    const type = stage === 'default' ? 'litter-fragments' : `litter-fragments-${stage}`;
    return { type, depth: 5, shade: .99 - .26 * wetness, saturation: .86 - .19 * wetness,
      shadow: stage === 'default' ? [34, 6, .28, 0, 2] : undefined };
  }
  const base = item.preset === 'heap' ? forestHeroStyles['leaf-gold'] : {
    type: 'leaf', depth: 5, mm: 55, flat: .67, shade: .92, saturation: .79, shadow: [71, 10, .49],
  };
  switch (leafPatchFrame(item)) {
    case 'default': return base;
    case 'worn': return { ...base, scale: (base.scale ?? 1) * .82, shade: .83, saturation: .64 };
    case 'trace': return { ...base, type: 'leaf', scale: (base.scale ?? 1) * .54, shade: .66, saturation: .57, opacity: .82 };
    case 'depleted': return;
  }
}

function terrainLayer(art: ComposedForestArt, item: ForestObject, draft: boolean, wetness = 0): ForestLayer {
  const anchor = forestStage(item.position);
  const mirror = item.mirrored ? -1 : 1;
  const style = physicalForestStyle(item, wetness);
  const layer: ForestLayer = {
    key: { kind: 'terrain', id: item.id }, anchor, mirror, parentMirror: 1,
    depth: style?.depth ?? 4, parts: [], terrain: item, draft,
  };
  if (style) layer.parts = [{ style, pose: forestPose(art, style, anchor, mirror) }];
  if (item.kind === 'barkShelter' && (item.preset === 'rootArch' || item.preset === 'branchBridge')) {
    const root = item.preset === 'rootArch';
    const name = root ? 'bridge-root' : 'bridge-branch';
    const mm = root ? 140 : 200;
    const foot = (root ? 51 : 78) * forestPixelsPerMm;
    const back: ForestStyle = { type: name, depth: 3, mm, flat: root ? 1 : .76, shade: .97, saturation: .86,
      contact: [[20, 9, .66, -foot, 1], [20, 9, .66, foot, 1]], shadow: [mm * .78, 13, .29, 0, -2] };
    const pose = forestPose(art, back, anchor, mirror);
    // The transparent opening is unchanged by dividing the raised span and the near feet.
    const division = root ? .66 : .60;
    layer.depth = back.depth;
    layer.parts = [{ style: back, pose, heightSlice: [0, division] }];
    layer.front = [{ style: back, pose, heightSlice: [division, 1] }];
  }
  if (item.kind === 'stonework') {
    const preset = item.preset === 'pair' || item.preset === 'cluster' ? item.preset : 'slab';
    layer.parts = (forestArt.stonework?.[preset].parts ?? []).map(({ offsetMm, lengthMm }, index) => {
      const style = { type: 'rock', depth: layer.depth, mm: lengthMm, shade: .88 + index * .05, flat: preset === 'slab' ? .54 : .90,
        shadow: [lengthMm * 1.45, lengthMm * .28, .50] };
      return { style, pose: forestPose(art, style, { ...anchor, x: anchor.x + offsetMm * forestPixelsPerMm * mirror }, mirror) };
    });
  }
  return layer;
}

function forestAnimal(art: ComposedForestArt, animal: FixtureAnimal, juvenile: boolean): PaintedPart {
  const base = forestHeroStyles['live-woodlouse'];
  const hidden = animal.activity === 'sheltering';
  const right = Math.cos(animal.heading) >= 0;
  const style = { ...base, flip: right, angle: Math.sin(animal.heading) * (right ? -.30 : .30),
    mm: juvenile ? 8 : 12, shade: hidden ? .78 : 1.30, shadow: juvenile ? [11, 3, .55] : base.shadow };
  return { style, pose: forestPose(art, style, forestStage(animal)) };
}

function forestVegetation(art: ComposedForestArt, snapshot: WorldSnapshot): ForestLayer[] {
  const plants = snapshot.forest?.life;
  if (!plants) return [];
  const result: ForestLayer[] = [];
  const allocatedMoss = [0, 0];
  const allocatedFern = [0, 0];
  for (const patch of snapshot.forestRocks?.mossPatches ?? []) allocatedMoss[patch.position.x < .5 ? 0 : 1] += patch.stockMgCnp[0];
  for (const patch of snapshot.forestRocks?.fernPatches ?? []) allocatedFern[patch.position.x < .5 ? 0 : 1] += patch.stockMgCnp[0];
  for (const side of [0, 1] as const) {
    // These subtle fixed patches mark the existing, side-aggregated live vegetation stock.
    const fern = Math.sqrt(clamp((plants.fernMgC[side] - allocatedFern[side]) / 90, 0, 1));
    const moss = Math.sqrt(clamp((plants.mossMgC[side] - allocatedMoss[side]) / 192, 0, 1));
    const fernStyle = { type: 'fern', depth: 0, mm: 54, opacity: fern * .47, shade: .56, flip: side === 1 };
    const mossStyle = { type: 'moss', depth: 1, mm: 42, opacity: moss * .66, shade: .65, flat: .75, flip: side === 1 };
    const atFern = { x: side === 0 ? 7 : 567, y: side === 0 ? 167 : 175 };
    const atMoss = { x: side === 0 ? 46 : 551, y: side === 0 ? 215 : 235 };
    if (fern > 0) result.push({ anchor: atFern, mirror: 1, parentMirror: 1, depth: fernStyle.depth,
      parts: [{ style: fernStyle, pose: forestPose(art, fernStyle, atFern) }] });
    if (moss > 0) result.push({ anchor: atMoss, mirror: 1, parentMirror: 1, depth: mossStyle.depth,
      parts: [{ style: mossStyle, pose: forestPose(art, mossStyle, atMoss) }] });
  }
  return result;
}

function keyOf({ kind, id }: ForestLayerKey) {
  return kind + '/' + id;
}

export function composedForestLayers(art: ComposedForestArt, snapshot: WorldSnapshot, terrain: ForestObject[],
  terrainDraft: ForestDraft | null, sceneryDraft: ForestSceneryDraft | null, fernDraft?: ForestFernDraft): ForestLayer[] {
  const scenery = snapshot.forestRocks?.scenery ?? [];
  const root: ForestLayer[] = forestVegetation(art, snapshot);
  for (const patch of snapshot.forestRocks?.mossPatches ?? []) {
    const initial = snapshot.forestRocks?.mossPreset;
    if (!initial || patch.stockMgCnp[0] <= 0) continue;
    const growth = clamp(Math.sqrt(patch.stockMgCnp[0] / initial.initialMgCnp[0]), .26, 1.38);
    const dryness = clamp(patch.stressFraction, 0, 1);
    const style = { type: 'moss', depth: 4.5, mm: initial.widthMm * growth, flat: .66,
      shade: .98 - dryness * .29, saturation: 1 - dryness * .63, opacity: .95 };
    const anchor = forestStage(patch.position);
    root.push({ key: { kind: 'moss', id: patch.id }, anchor, depth: style.depth, mirror: 1, parentMirror: 1,
      parts: [{ style, pose: forestPose(art, style, anchor) }] });
  }
  const fernPreset = snapshot.forestRocks?.fernPreset;
  if (fernPreset) {
    for (const patch of snapshot.forestRocks?.fernPatches ?? []) {
      if (patch.stockMgCnp[0] <= 0) continue;
      const moving = fernDraft?.id === patch.id ? fernDraft.position : undefined;
      const anchor = forestStage(moving ?? patch.position);
      const part = fernPart(art, patch, fernPreset, anchor);
      root.push({ key: { kind: 'fern', id: patch.id }, anchor, depth: part.style.depth, mirror: 1, parentMirror: 1,
        parts: [part], draft: Boolean(moving) });
    }
    if (fernDraft?.id === null) {
      const anchor = forestStage(fernDraft.position);
      const part = fernPart(art, { stockMgCnp: fernPreset.initialMgCnp, stressFraction: 0 }, fernPreset, anchor);
      root.push({ key: { kind: 'fern', id: -1 }, anchor, depth: part.style.depth, mirror: 1, parentMirror: 1, parts: [part], draft: true });
    }
  }
  const children = new Map<string, ForestScenery[]>();
  for (const item of scenery) {
    if (!item.parent) continue;
    const key = keyOf(item.parent);
    const group = children.get(key) ?? [];
    group.push(item);
    children.set(key, group);
  }

  function decoration(item: ForestScenery, parent?: ForestLayer): ForestLayer | undefined {
    const style = forestSceneryStyles.get(item.style);
    if (!style) return;
    const draft = sceneryDraft?.id === item.id ? sceneryDraft : undefined;
    const local = forestStage(draft ?? item);
    const mirror = parent?.mirror ?? 1;
    const at = parent ? { x: parent.anchor.x + local.x * mirror, y: parent.anchor.y + local.y } : local;
    const group = seedShellGroup(style.seedShellGroup);
    const parts = group ? group.pieces.map((piece) => {
      const { x, y, mm, angle, flip } = piece;
      const anchor = { x: at.x + x * forestPixelsPerMm, y: at.y + y * forestPixelsPerMm };
      const pieceStyle = { type: style.type, depth: style.depth, mm, angle, flip, shade: .86, saturation: .83,
        shadow: [mm * forestPixelsPerMm * .91, Math.max(1.5, mm * .24), .5] };
      return { style: pieceStyle, pose: forestPose(art, pieceStyle, anchor) };
    }) : [{ style, pose: forestPose(art, style, at, mirror) }];
    return {
      key: { kind: 'scenery', id: item.id }, depth: style.depth, anchor: at, parent: item.parent,
      parentMirror: mirror, mirror, draft: Boolean(draft),
      parts,
      invalidSite: Boolean(group && !isSeedShellSite(group, draft ?? item)),
    };
  }

  function wetness(position: Point) {
    const cell = snapshot.forestRocks?.substrateCells?.find(({ cellIndex }) => cellIndex === forestSoilIndex(position));
    return cell && cell.waterCapacityMl > 0 ? clamp(cell.waterMl / cell.waterCapacityMl, 0, 1) : 0;
  }
  for (const item of terrain) {
    const draft = terrainDraft?.id === item.id ? terrainDraft : null;
    const layer = terrainLayer(art, draft ? { ...item, ...draft, id: item.id } : item, Boolean(draft),
      item.preset === 'fragments' ? wetness(draft?.position ?? item.position) : 0);
    if (!draft) layer.mycelium = observeForestMycelium(snapshot.forestRocks?.substrateCells, item);
    root.push(layer);
  }
  if (terrainDraft?.id === null) root.push(terrainLayer(art, { ...terrainDraft, id: -1 }, true,
    terrainDraft.preset === 'fragments' ? wetness(terrainDraft.position) : 0));
  for (const item of scenery) {
    if (item.parent) continue;
    const layer = decoration(item);
    if (layer) root.push(layer);
  }
  if (sceneryDraft?.id === null) {
    const layer = decoration({ ...sceneryDraft, id: -1 });
    if (layer) root.push({ ...layer, draft: true });
  }

  const shelter = new Map<string, ForestLayer>();
  for (const layer of root) {
    for (const id of layer.terrain?.occupantIds ?? []) shelter.set(id, layer);
  }
  const solid = root.filter(layer => layer.terrain && layer.terrain.kind !== 'leafPatch');
  const juveniles = new Set(snapshot.forest?.life?.woodlice.filter(({ stage }) => stage === 'juvenile').map(({ id }) => id));
  for (const animal of snapshot.animals) {
    const part = forestAnimal(art, animal, juveniles.has(animal.id));
    const cover = shelter.get(animal.id);
    if (cover) {
      // Keep actual animal position under the bark alpha. Only pixels in the open arch remain visible.
      (cover.inside ??= []).push(part);
    } else {
      const near = solid.some(layer => Math.abs(layer.anchor.x - part.pose.at.x) < 100 && part.pose.at.y <= layer.anchor.y);
      root.push({ anchor: part.pose.at, mirror: 1, parentMirror: 1, depth: near ? 3 : 5.5, parts: [part] });
    }
  }
  const ordered: ForestLayer[] = [];
  function group(layer: ForestLayer) {
    ordered.push(layer);
    if (!layer.key) return;
    const nested = (children.get(keyOf(layer.key)) ?? []).map(child => decoration(child, layer))
      .filter(child => child !== undefined).sort((a, b) => a.depth - b.depth);
    for (const child of nested) group(child);
  }
  root.sort((a, b) => a.depth - b.depth || a.anchor.y - b.anchor.y);
  for (const layer of root) group(layer);
  return ordered;
}

export function pickComposedForest(layers: readonly ForestLayer[], point: Point): ForestLayer | undefined {
  for (let index = layers.length - 1; index >= 0; index--) {
    const item = layers[index];
    if (!item.draft && item.key && item.parts.some(({ pose }) => hitsForestSprite(point, pose))) return item;
  }
}

function placementCells(ctx: CanvasRenderingContext2D, hint?: PlacementHint) {
  for (const cell of hint?.cells ?? []) {
    const { x, y } = forestStage(cell);
    ctx.strokeStyle = cell.occupied ? '#ffd39d'
      : cell.role === 'passage' ? '#b5def4' : cell.role === 'substrate' ? '#e2c79c' : '#b6eddb';
    ctx.setLineDash(cell.role === 'passage' ? [3, 2] : []);
    ctx.strokeRect(x + .5, y + .5, cell.width * forestWidth - 1, cell.height * forestHeight - 1);
  }
  ctx.setLineDash([]);
}

export function drawComposedForest(ctx: CanvasRenderingContext2D, art: ComposedForestArt, snapshot: WorldSnapshot,
  layers: ForestLayer[], selection?: ForestLayerKey, hint?: PlacementHint, springtails?: ForestSpringtailStock) {
  ctx.save();
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  paintComposedForestBed(ctx, art.bed);
  paintForestSubstrate(ctx, snapshot.forestRocks?.substrateCells);
  if (springtails) paintForestSpringtails(ctx, springtails);
  for (const layer of layers) {
    ctx.save();
    if (layer.draft && (layer.key?.kind !== 'terrain' || hint?.site !== 'ground')) ctx.globalAlpha = .71;
    for (const { style, pose } of layer.parts) drawForestContact(ctx, style, pose, layer.mirror);
    const groups = layer.front ? [layer.parts, layer.inside, layer.front] : [layer.inside, layer.parts];
    for (const group of groups) {
      for (const { style, pose, heightSlice } of group ?? []) drawForestSprite(ctx, style, pose, heightSlice);
    }
    if (layer.mycelium && layer.parts[0]) paintForestMycelium(ctx, layer.parts[0].pose, layer.mycelium);
    ctx.restore();
  }
  paintComposedForestLight(ctx, snapshot.daylightPhase, snapshot.season?.dayOfYear);
  for (const layer of layers) {
    if (!layer.draft && !sameForestLayer(layer.key, selection)) continue;
    const rejected = layer.draft && (layer.invalidSite || (layer.key?.kind === 'terrain' && hint?.site !== 'ground'));
    const color = rejected ? '#ffd39d' : '#d7efcf';
    for (const { pose } of layer.parts) markForestSprite(ctx, pose, color, Boolean(layer.draft));
  }
  placementCells(ctx, hint);
  ctx.restore();
}
