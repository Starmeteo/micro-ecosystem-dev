import type { PondBasinSnapshot, PondSedgePatch } from '../../world';
import type { Rect } from './geometry';
import { composedPixelsPerMm, composedWidth, type ComposedArt } from './pondComposedArt';

interface SedgeVisual {
  patch: PondSedgePatch;
  x: number;
  bedY: number;
  collarY: number;
  rootsHeight: number;
  rootWidth: number;
  bladeHeight: number;
  foreground: boolean;
}

export function pondSedgeVisuals(pond: PondBasinSnapshot, bed: number, surfaceAtMaximum: number): SedgeVisual[] {
  if (!pond.shape || !pond.shoreSedges?.length) return [];
  const { columns, protectedRow } = pond.shape;
  const projectionY = (bed - surfaceAtMaximum) / (pond.maxDepthCm * 10);
  return pond.shoreSedges.map(patch => {
    const { waterCell, shoreCell } = patch.anchor;
    const row = (Math.floor(waterCell / columns) + Math.floor(shoreCell / columns)) / 2;
    const column = (waterCell % columns + shoreCell % columns + 1) / (2 * columns);
    const depth = Math.min(1, Math.max(-1, (row - protectedRow) / 6));
    const x = composedWidth * (.5 + (column - .5) * (1 + .13 * depth));
    return { patch, x, bedY: bed, collarY: bed - patch.rootCollarMm * projectionY,
      rootsHeight: patch.rootDiameterMm / 2 * projectionY, rootWidth: patch.rootDiameterMm * composedPixelsPerMm,
      bladeHeight: patch.shootHeightMm * composedPixelsPerMm, foreground: depth >= 0 };
  }).sort((a, b) => Number(a.foreground) - Number(b.foreground) || a.x - b.x || a.patch.id - b.patch.id);
}

const bankSurfaces = new WeakMap<ComposedArt, Map<string, HTMLCanvasElement>>();

function bankSurface(art: ComposedArt, width: number, drop: number, rootReach: number) {
  const cache = bankSurfaces.get(art) ?? new Map<string, HTMLCanvasElement>();
  bankSurfaces.set(art, cache);
  const key = `${width}/${drop}/${rootReach}`;
  const existing = cache.get(key);
  if (existing) return existing;
  const roots = art.sprites.get('sedge-bank-roots');
  if (!roots) return undefined;
  const ground = document.createElement('canvas');
  ground.width = width;
  ground.height = drop + 18;
  const context = ground.getContext('2d');
  if (!context) return undefined;
  const h = ground.height;
  const w = ground.width;
  const join = 8;
  const toe = drop + 13;
  context.imageSmoothingQuality = 'high';
  // Fill the slope below the exposed roots with the same textured sand as the actual bed.
  // Its irregular shoulders meet the bed below the root tip and the apron fades into it.
  context.save();
  context.beginPath();
  context.moveTo(w * .49, join + 1);
  context.lineTo(w * .44, join + rootReach * .13);
  context.lineTo(w * .37, join + rootReach * .28);
  context.lineTo(w * .33, h * .43);
  context.lineTo(w * .24, h * .56);
  context.lineTo(w * .22, h * .65);
  context.lineTo(w * .11, h * .76);
  context.lineTo(w * .03, toe - 2);
  context.lineTo(w * .16, toe + 3);
  context.lineTo(w * .76, toe + 2);
  context.lineTo(w * .98, toe - 5);
  context.lineTo(w * .84, h * .73);
  context.lineTo(w * .76, h * .62);
  context.lineTo(w * .69, h * .56);
  context.lineTo(w * .65, h * .46);
  context.lineTo(w * .57, join + rootReach * .26);
  context.lineTo(w * .54, join + rootReach * .12);
  context.closePath();
  context.clip();
  const { naturalWidth, naturalHeight } = art.bed;
  context.drawImage(art.bed, naturalWidth * .26, naturalHeight * .73, naturalWidth * .46, naturalHeight * .26, 0, 0, w, h);
  const shade = context.createLinearGradient(0, join, w, h);
  shade.addColorStop(0, 'rgba(34,43,32,.45)');
  shade.addColorStop(.48, 'rgba(22,38,32,.33)');
  shade.addColorStop(1, 'rgba(45,34,18,.52)');
  context.fillStyle = shade;
  context.fillRect(0, 0, w, h);
  context.restore();

  // Crown at (1033,280) in the authored bank crop. This aligns it with the leaf sprite.
  // Only the root-carrying layer is scaled by actual root depth, not the connecting ground.
  const rootTextureWidth = w * .86;
  const rootTextureHeight = rootReach * 1.09;
  const exposedRoots = document.createElement('canvas');
  exposedRoots.width = w;
  exposedRoots.height = h;
  const rootLayer = exposedRoots.getContext('2d');
  if (rootLayer) {
    rootLayer.imageSmoothingQuality = 'high';
    rootLayer.drawImage(roots.image, 10, 249, 2010, 462, w * .5 - rootTextureWidth * (1023 / 2010),
      join - rootTextureHeight * (31 / 462), rootTextureWidth, rootTextureHeight);
    rootLayer.globalCompositeOperation = 'destination-in';
    // The pale upper root fibers stay distinct while the lower soil fades into the bed apron.
    const rootedSand = rootLayer.createLinearGradient(0, join + rootReach * .62, 0, join + rootReach * .98);
    rootedSand.addColorStop(0, '#fff');
    rootedSand.addColorStop(.48, '#ffffffab');
    rootedSand.addColorStop(1, 'transparent');
    rootLayer.fillStyle = rootedSand;
    rootLayer.fillRect(0, 0, w, h);
    context.drawImage(exposedRoots, 0, 0);
  }
  context.globalCompositeOperation = 'destination-in';
  const apronFade = context.createLinearGradient(0, 0, 0, h);
  apronFade.addColorStop(0, '#fff');
  apronFade.addColorStop(.81, '#fff');
  apronFade.addColorStop(.94, 'rgba(255,255,255,.55)');
  apronFade.addColorStop(1, 'transparent');
  context.fillStyle = apronFade;
  context.fillRect(0, 0, w, h);
  cache.set(key, ground);
  return ground;
}

function paintSedge(context: CanvasRenderingContext2D, art: ComposedArt, plant: SedgeVisual, underwater: boolean) {
  const { patch, x, collarY, bedY, rootWidth, rootsHeight, bladeHeight } = plant;
  const leaves = art.sprites.get('sedge-shoot');
  const width = Math.ceil(Math.max(rootWidth * 1.98, (bedY - collarY) * .91));
  const base = bankSurface(art, width, Math.ceil(bedY - collarY), Math.ceil(rootsHeight));
  context.save();
  const rx = width * .46;
  const ry = (plant.foreground ? 9 : 7);
  context.save();
  context.translate(x + width * .025, bedY + 4);
  context.scale(1, ry / rx);
  const shadow = context.createRadialGradient(0, 0, 0, 0, 0, rx);
  shadow.addColorStop(0, underwater ? 'rgba(17,32,29,.66)' : 'rgba(39,30,20,.59)');
  shadow.addColorStop(.55, underwater ? 'rgba(17,32,29,.38)' : 'rgba(39,30,20,.31)');
  shadow.addColorStop(1, 'transparent');
  context.fillStyle = shadow;
  context.fillRect(-rx, -rx, rx * 2, rx * 2);
  context.restore();
  if (base) {
    context.filter = underwater ? 'brightness(.70) saturate(.70)' : 'brightness(.93) saturate(.84)';
    context.drawImage(base, x - width * .5, collarY - 8);
  }
  if (leaves) {
    const height = bladeHeight;
    const width = height * .55;
    const flip = patch.id % 2 === 0 ? -1 : 1;
    context.translate(x, collarY + 1);
    context.scale(flip, 1);
    context.filter = underwater ? 'brightness(.65) saturate(.65)' : patch.rootWetFraction > 0
      ? 'brightness(.96) saturate(.91)' : 'brightness(.88) saturate(.63)';
    // Crop only outer alpha. Top visible blade to common collar is the selected reference height.
    context.drawImage(leaves.image, 31, 152, 946, 1248, -width * .483, -height, width, height);
  }
  context.restore();
}

export function paintPondSedges(context: CanvasRenderingContext2D, art: ComposedArt, camera: Rect, plants: readonly SedgeVisual[],
  wet: boolean, surface: number, foreground: boolean) {
  const group = plants.filter(plant => plant.foreground === foreground);
  if (!group.length) return;
  const edge = Math.max(camera.y, Math.min(camera.y + camera.height, surface));
  const water = wet ? camera.y + camera.height - edge : 0;
  if (water > 0) {
    context.save();
    context.beginPath();
    context.rect(camera.x, edge, camera.width, water);
    context.clip();
    for (const plant of group) paintSedge(context, art, plant, true);
    context.restore();
  }
  if (!wet || edge > camera.y) {
    context.save();
    context.beginPath();
    context.rect(camera.x, camera.y, camera.width, wet ? edge - camera.y : camera.height);
    context.clip();
    for (const plant of group) paintSedge(context, art, plant, false);
    context.restore();
  }
}

export function markPondSedge(context: CanvasRenderingContext2D, plants: readonly SedgeVisual[], selectedId?: number) {
  const plant = plants.find(({ patch }) => patch.id === selectedId);
  if (!plant) return;
  context.save();
  context.beginPath();
  context.ellipse(plant.x, plant.collarY + 4, plant.rootWidth * .61 + 2, Math.max(7, plant.rootsHeight * .43), 0, 0, Math.PI * 2);
  context.strokeStyle = '#c0f0df';
  context.lineWidth = 1.4;
  context.stroke();
  context.restore();
}
