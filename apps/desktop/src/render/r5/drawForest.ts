import type { FixtureAnimal, WorldSnapshot } from '../../world';
import { seasonColor } from '../seasonColor';
import { R5_HEIGHT, R5_WIDTH, type Point, clamp, point, sampleProjection, sceneNormalised, scenePixels } from './geometry';
import { type SampleImages, drawArtAt, hitSprite, placementHit } from './sceneArt';
import { type ForestLandscape, paintForestLandscape } from './forestLandscape';
import {
  type ForestDraft, type ForestObject, type ForestShape, type PlacementHint, forestArt, forestObjectArt, leafPatchFrame,
} from './forestPlacement';

const { objects, layers, referenceWidthMm } = forestArt;
const woodlouse = objects.woodlouse!;
const [head, tail] = woodlouse.body;
const adultScale = sampleProjection(woodlouse.lengthMm, referenceWidthMm, R5_WIDTH)
  / Math.hypot(tail[0] - head[0], tail[1] - head[1]);

function individualStyle(animal: FixtureAnimal, juvenile: boolean) {
  const sheltered = animal.activity === 'sheltering';
  const moving = ['moving', 'foraging', 'feeding'].includes(animal.activity);
  return {
    position: scenePixels(animal),
    sprite: sheltered ? woodlouse.sprites.hidden : moving ? woodlouse.sprites.walk : woodlouse.sprites.default,
    scale: juvenile ? adultScale * .66 : adultScale,
    mirrored: Math.cos(animal.heading) < 0,
  };
}

export function forestAxis(snapshot: WorldSnapshot): readonly [Point, Point] | undefined {
  const first = snapshot.animals[0];
  if (!first) return;
  const juvenile = snapshot.forest?.life?.woodlice.some(({ id, stage }) => id === first.id && stage === 'juvenile') ?? false;
  const { position, scale, mirrored } = individualStyle(first, juvenile);
  const endpoint = ([x, y]: readonly [number, number]) => ({
    x: position.x + (x - woodlouse.anchor[0]) * scale * (mirrored ? -1 : 1),
    y: position.y + (y - woodlouse.anchor[1]) * scale,
  });
  return [endpoint(head), endpoint(tail)];
}

export function forestObjects(snapshot: WorldSnapshot): ForestObject[] {
  const layout = snapshot.forestRocks;
  const entries: ForestObject[] = [
    ...(layout?.rocks ?? []).map((rock): ForestObject => ({ ...rock, kind: 'rock' })),
    ...(layout?.logs ?? []).map((log): ForestObject => ({ ...log, kind: 'log' })),
    ...(layout?.stonework ?? []).map((shape): ForestObject => ({ ...shape, kind: 'stonework' })),
    ...(layout?.barkShelters ?? []).map((shelter): ForestObject => ({ ...shelter, kind: 'barkShelter' })),
    ...(layout?.leafPatches ?? []).map((patch): ForestObject => ({ ...patch, kind: 'leafPatch' })),
  ];
  const groundLayer = (entry: ForestObject) => entry.kind === 'leafPatch' ? 0 : 1;
  return entries.sort((a, b) => groundLayer(a) - groundLayer(b) || a.position.y - b.position.y || a.id - b.id);
}

export function forestObjectAt(
  entries: readonly ForestObject[], at: Point, images: SampleImages, precise: boolean, showScenicLargeLeaf: boolean,
  landscape = false,
) {
  if (precise && !landscape) {
    const foreground = images.get(layers.foreground);
    if (foreground && hitSprite(foreground, Math.floor(at.x), Math.floor(at.y))) return;
    const leaf = showScenicLargeLeaf ? objects.leafLarge : undefined;
    if (leaf && placementHit(leaf, point(leaf.position), at, images, false, 0)) return;
  }
  for (let index = entries.length - 1; index >= 0; index--) {
    const item = entries[index];
    const asset = forestObjectArt(item);
    const art = item.kind === 'leafPatch' ? { ...asset, sprites: { default: asset.sprites[leafPatchFrame(item)] } } : asset;
    if (placementHit(art, scenePixels(item.position), at, images, item.mirrored, precise ? 0 : 3)) return item;
  }
}

function outline(
  ctx: CanvasRenderingContext2D, shape: ForestShape, position: Point, mirrored: boolean, color: string, dashed = false,
) {
  const art = forestObjectArt(shape);
  ctx.save();
  ctx.translate(position.x, position.y);
  ctx.scale(mirrored ? -1 : 1, 1);
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  if (dashed) ctx.setLineDash([3, 3]);
  ctx.strokeRect(-art.anchor[0] + .5, -art.anchor[1] + .5, art.width - 1, art.height - 1);
  ctx.restore();
}

export function drawForest(
  canvas: HTMLCanvasElement, images: SampleImages, snapshot: WorldSnapshot, entries: readonly ForestObject[],
  chosen: number | null, draft: ForestDraft | null, hint: PlacementHint | null, landscape?: ForestLandscape,
) {
  const ctx = canvas.getContext('2d', { alpha: false });
  if (!ctx) return;
  ctx.setTransform(canvas.width / R5_WIDTH, 0, 0, canvas.height / R5_HEIGHT, 0, 0);
  ctx.clearRect(0, 0, R5_WIDTH, R5_HEIGHT);
  ctx.imageSmoothingEnabled = false;
  if (landscape) paintForestLandscape(ctx, landscape, snapshot);
  else ctx.drawImage(images.get(layers.base)!, 0, 0);

  const elevation = Math.cos((snapshot.daylightPhase - .5) * Math.PI * 2);
  const daylight = clamp((elevation + .17) / .65, 0, 1);
  const leaf = objects.leaf;
  if (!landscape && leaf) drawArtAt(ctx, images, leaf, point(leaf.position), leaf.sprites.default);
  const materialLayers = landscape?.materials ?? images;

  const drawStructure = (
    item: Omit<ForestDraft, 'id'> & Pick<ForestObject, 'initialMgCnp' | 'remainingMgCnp'>,
    shadow = false, drawInside?: () => void,
  ) => {
    const art = forestObjectArt(item);
    const position = scenePixels(item.position);
    if (shadow) {
      drawArtAt(ctx, materialLayers, art, position, art.sprites.shadow, 1, item.mirrored);
      return;
    }
    if (item.kind === 'leafPatch') {
      drawArtAt(ctx, materialLayers, art, position, art.sprites[leafPatchFrame(item)], 1, item.mirrored);
      return;
    }
    for (const sprite of [art.sprites.interior, art.sprites.back ?? art.sprites.default]) {
      drawArtAt(ctx, materialLayers, art, position, sprite, 1, item.mirrored);
    }
    drawInside?.();
    for (const sprite of [art.sprites.front, art.sprites.growth]) {
      drawArtAt(ctx, materialLayers, art, position, sprite, 1, item.mirrored);
    }
  };

  if (!landscape && !Array.isArray(snapshot.forestRocks?.logs)) {
    const log = objects.log;
    const decoration = { kind: 'log', position: sceneNormalised(point(log.position)), mirrored: false } as const;
    drawStructure(decoration, true);
    drawStructure(decoration);
    const moss = objects.moss;
    if (moss) {
      const mass = snapshot.forest?.life?.mossMgC[0];
      ctx.save();
      ctx.globalAlpha = mass === undefined ? 1 : Math.sqrt(clamp(mass / 192, 0, 1));
      drawArtAt(ctx, images, moss, point(moss.position), moss.sprites.default);
      ctx.restore();
    }
  }

  const visible = entries.filter((entry) => entry.id !== draft?.id);
  for (const item of visible) {
    if (item.kind === 'leafPatch') drawStructure(item);
    else drawStructure(item, true);
  }
  const juveniles = new Set(snapshot.forest?.life?.woodlice.filter(({ stage }) => stage === 'juvenile').map(({ id }) => id));
  const drawIndividual = (animal: FixtureAnimal) => {
    const { position, sprite, scale, mirrored } = individualStyle(animal, juveniles.has(animal.id));
    ctx.fillStyle = 'rgba(16,20,12,.42)';
    ctx.beginPath();
    ctx.ellipse(position.x, position.y + 1, 5 * scale, 1.25 * scale, 0, 0, Math.PI * 2);
    ctx.fill();
    drawArtAt(ctx, images, woodlouse, position, sprite, scale, mirrored);
  };
  type DrawEntry = { y: number; animal: FixtureAnimal } | { y: number; structure: ForestObject };
  const drawOrder: DrawEntry[] = visible.filter(({ kind }) => kind !== 'leafPatch')
    .map((structure) => ({ y: structure.position.y, structure }));
  const animalsById = new Map(snapshot.animals.map((animal) => [animal.id, animal]));
  // A cover relation is a rendering layer assignment. The snapshot's position is kept.
  const insideCover = new Set(visible.flatMap(({ occupantIds }) => occupantIds ?? []));
  for (const animal of snapshot.animals) {
    if (insideCover.has(animal.id)) continue;
    if (animal.activity === 'sheltering') drawIndividual(animal);
    else drawOrder.push({ y: animal.y, animal });
  }
  drawOrder.sort((a, b) => a.y - b.y);
  for (const item of drawOrder) {
    if ('animal' in item) drawIndividual(item.animal);
    else drawStructure(item.structure, false, () => {
      for (const id of item.structure.occupantIds ?? []) {
        const animal = animalsById.get(id);
        if (animal) drawIndividual(animal);
      }
    });
  }

  if (!landscape) {
    const foregroundLeaf = objects.leafLarge;
    if (foregroundLeaf && !Array.isArray(snapshot.forestRocks?.leafPatches)) {
      drawArtAt(ctx, images, foregroundLeaf, point(foregroundLeaf.position), foregroundLeaf.sprites.default);
    }
    ctx.drawImage(images.get(layers.foreground)!, 0, 0);
    const light = layers.light ? images.get(layers.light) : undefined;
    if (light) {
      ctx.save();
      ctx.globalAlpha = daylight;
      ctx.drawImage(light, 0, 0);
      ctx.restore();
    }
  }
  const season = seasonColor(snapshot.season?.dayOfYear, 'light');
  const night = landscape ? [111, 135, 172] : [92, 114, 157];
  const tintedDay = landscape ? season.map(channel => 255 - .57 * (255 - channel)) : season;
  const tint = night.map((channel, index) => Math.round(channel + (tintedDay[index] - channel) * daylight));
  ctx.save();
  ctx.globalCompositeOperation = 'multiply';
  ctx.fillStyle = `rgb(${tint.join(',')})`;
  ctx.fillRect(0, 0, R5_WIDTH, R5_HEIGHT);
  ctx.restore();

  const selected = visible.find((entry) => entry.id === chosen);
  if (selected) outline(ctx, selected, scenePixels(selected.position), selected.mirrored, '#b6eddb');
  if (draft) {
    const position = scenePixels(draft.position);
    ctx.save();
    ctx.globalAlpha = .68;
    // Moving a depleted patch previews its current cel. Adding previews the selected stock preset.
    const current = entries.find(({ id }) => id === draft.id);
    drawStructure({ ...current, ...draft });
    ctx.restore();
    const color = hint?.site === 'ground' ? '#d3e9ea' : '#ffd39d';
    outline(ctx, draft, position, draft.mirrored, color, true);
    if (hint) for (const cell of hint.cells) {
      const { x, y } = scenePixels(cell);
      const passage = cell.role === 'passage';
      ctx.strokeStyle = cell.occupied ? '#ffd39d' : passage ? '#b5def4' : cell.role === 'substrate' ? '#e2c79c' : '#b6eddb';
      ctx.setLineDash(passage ? [3, 2] : []);
      ctx.strokeRect(x + .5, y + .5, cell.width * R5_WIDTH - 1, cell.height * R5_HEIGHT - 1);
    }
    ctx.setLineDash([]);
    if (draft.kind === 'log') {
      const halfSpan = sampleProjection(objects.log.lengthMm, referenceWidthMm, R5_WIDTH) / 2;
      ctx.beginPath();
      ctx.moveTo(position.x - halfSpan, position.y);
      ctx.lineTo(position.x + halfSpan, position.y);
      ctx.strokeStyle = color;
      ctx.stroke();
    }
  }
}
