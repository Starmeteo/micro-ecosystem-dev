import definitions from '../assets/r5/pond-composed/definition.json';
import type { Point, Rect } from './geometry';
import type { PondOpticsArt } from './pondComposedOptics';
import { pondStructureProfile, type PondStructurePreset } from './pondStructureProfiles';

export const composedWidth = 576;
export const composedHeight = 324;
export const composedPixelsPerMm = composedWidth / 600 * 1.69;
const source = '../assets/r5/pond-composed/';
const urls = import.meta.glob<string>('../assets/r5/pond-composed/*.png', { eager: true, query: '?url', import: 'default' });
const structureFiles = {
  hangingRoots: 'hanging-roots.png', rootNetwork: 'root-network.png', flatStoneCave: 'flat-stone-cave.png',
} satisfies Record<PondStructurePreset, string>;

export interface SceneryStyle {
  id?: string;
  label?: string;
  type: string;
  band: number;
  attachBand?: number;
  mm?: number;
  flat?: number;
  flip?: boolean;
  angle?: number;
  shade?: number;
  saturation?: number;
  opacity?: number;
  rootCut?: boolean;
  rings?: boolean;
  bedAnchor?: { x: readonly number[]; y: readonly number[] };
  shadow?: readonly number[];
  extraShadow?: { local: readonly number[]; size: readonly number[] };
}

interface Sprite {
  image: HTMLImageElement | HTMLCanvasElement;
  w: number;
  h: number;
  mm: number;
  axis: number[][];
  anchor: number[];
  projection: number;
  mask: ImageData;
}

export interface SpritePose extends Point {
  art: Sprite;
  scale: number;
  angle: number;
  flip: number;
  flat: number;
}

export type ComposedArt = PondOpticsArt & { sprites: Map<string, Sprite> };
export const sceneryStyles = new Map<string, SceneryStyle>(definitions.styles.map((style) => [style.id, style]));
export const composedPhysicalStyles = definitions.physical;

async function loadImage(file: string) {
  const url = urls[source + file];
  if (!url) throw new Error('Missing composed pond asset ' + file);
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

function layerFor(w: number, h: number) {
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('Pond V2 requires Canvas 2D');
  return context;
}

async function loadSprite(spec: typeof definitions.art[keyof typeof definitions.art]): Promise<Sprite> {
  const image = await loadImage(spec.file);
  const { naturalWidth: w, naturalHeight: h } = image;
  const maskScale = Math.min(1, 460 / Math.max(w, h));
  const layer = layerFor(Math.ceil(w * maskScale), Math.ceil(h * maskScale));
  layer.drawImage(image, 0, 0, layer.canvas.width, layer.canvas.height);
  return { ...spec, image, w, h, mask: layer.getImageData(0, 0, layer.canvas.width, layer.canvas.height) };
}

async function loadStructureSprites(preset: PondStructurePreset, file: string): Promise<[string, Sprite][]> {
  const profile = pondStructureProfile(preset);
  if (!profile) throw new Error(`Missing pond structure profile ${preset}`);
  const image = await loadImage(file);
  const sample = layerFor(448, Math.ceil(448 * image.naturalHeight / image.naturalWidth));
  const { width, height } = sample.canvas;
  sample.imageSmoothingQuality = 'high';
  sample.drawImage(image, 0, 0, width, height);
  const data = sample.getImageData(0, 0, width, height).data;
  let minX = width, minY = height, maxX = -1, maxY = -1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (data[(y * width + x) * 4 + 3] <= 24) continue;
      minX = Math.min(minX, x); maxX = Math.max(maxX, x);
      minY = Math.min(minY, y); maxY = Math.max(maxY, y);
    }
  }
  if (maxX < minX || maxY < minY) throw new Error(`Empty pond structure sprite ${file}`);
  const w = maxX - minX + 1;
  const h = maxY - minY + 1;
  const source = layerFor(w, h);
  if (preset === 'flatStoneCave') {
    // Align the authored roof underside to the measured band; lower supports sit behind the swim plane.
    const sourceRoof = Math.round(h * .51);
    const underside = profile.bands.find(({ left, right }) => left[0] <= 0 && right[0] >= 0)?.left[1];
    if (underside === undefined) throw new Error('Missing span above pond cave opening');
    const roof = Math.round(h * (profile.top - underside) / (profile.top - profile.bottom));
    source.drawImage(sample.canvas, minX, minY, w, sourceRoof, 0, 0, w, roof);
    source.drawImage(sample.canvas, minX, minY + sourceRoof, w, h - sourceRoof, 0, roof, w, h - roof);
  } else {
    source.drawImage(sample.canvas, minX, minY, w, h, 0, 0, w, h);
  }
  const solidMask = layerFor(w, h);
  const xAt = (x: number) => (x - profile.left) * w / (profile.right - profile.left);
  const yAt = (y: number) => (profile.top - y) * h / (profile.top - profile.bottom);
  for (const { left: [lx, lb, lt], right: [rx, rb, rt] } of profile.bands) {
    solidMask.beginPath();
    solidMask.moveTo(xAt(lx), yAt(lt));
    solidMask.lineTo(xAt(rx), yAt(rt));
    solidMask.lineTo(xAt(rx), yAt(rb));
    solidMask.lineTo(xAt(lx), yAt(lb));
    solidMask.closePath();
    solidMask.fill();
  }
  const spec = { w, h, mm: 1, projection: 1, axis: [[0, .5], [1, .5]], anchor: [-profile.left / (profile.right - profile.left), 1] };
  return (['rear', 'front'] as const).map((pass) => {
    const context = layerFor(w, h);
    context.drawImage(source.canvas, 0, 0);
    context.globalCompositeOperation = pass === 'front' ? 'destination-in' : 'destination-out';
    context.drawImage(solidMask.canvas, 0, 0);
    return [`${preset}-${pass}`, { ...spec, image: context.canvas, mask: context.getImageData(0, 0, w, h) }];
  });
}

let decoded: Promise<ComposedArt> | undefined;
export function loadComposedArt() {
  const files = definitions.environment;
  decoded ??= Promise.all([
    loadImage(files.bed), loadImage(files.caustics), loadImage(files.surface), loadImage(files.mirror),
    Promise.all(Object.entries(definitions.art).map(async ([id, spec]) => [id, await loadSprite(spec)] as const)),
    Promise.all(Object.entries(structureFiles).map(([preset, file]) => loadStructureSprites(preset as PondStructurePreset, file))),
  ]).then(([bed, caustics, surface, mirrorSource, entries, structures]) => {
    const mirror = layerFor(composedWidth * 2, 144);
    mirror.drawImage(mirrorSource, 0, 0, mirror.canvas.width, mirror.canvas.height);
    mirror.globalCompositeOperation = 'destination-in';
    const falloff = mirror.createLinearGradient(0, 0, 0, mirror.canvas.height);
    falloff.addColorStop(0, '#fff');
    falloff.addColorStop(.56, '#ffffffe0');
    falloff.addColorStop(1, '#ffffff00');
    mirror.fillStyle = falloff;
    mirror.fillRect(0, 0, mirror.canvas.width, mirror.canvas.height);
    const art = { bed, caustics, surface, mirror: mirror.canvas, sprites: new Map<string, Sprite>([...entries, ...structures.flat()]) };
    registerGravelPatch(art);
    return art;
  }).catch((err: unknown) => {
    decoded = undefined;
    throw err;
  });
  return decoded;
}

// Assemble the authored grain arrangement from independent pebble art, leaving the part outside each grain transparent.
function registerGravelPatch(art: ComposedArt) {
  const { type, source: grain, spanMm, projectedDepthMm, grains } = definitions.gravelPatch;
  const supersample = 2;
  const w = Math.ceil(spanMm * composedPixelsPerMm * supersample);
  const h = Math.ceil(projectedDepthMm * composedPixelsPerMm * supersample);
  const context = layerFor(w, h);
  context.scale(supersample, supersample);
  context.imageSmoothingQuality = 'high';
  for (const [index, [x, y, mm, angle]] of grains.entries()) {
    const style = { type: grain, band: 1, mm, angle, flat: .79, flip: index % 2 === 1,
      shade: .69 + index % 4 * .09, saturation: .61, opacity: .95 };
    const pose = spritePose(art, style, { x: w / (2 * supersample) + x * composedPixelsPerMm,
      y: h / (2 * supersample) + y * composedPixelsPerMm });
    drawShadow(context, pose, [mm * 1.6, mm * .38, .32]);
    drawScenerySprite(context, style, pose);
  }
  art.sprites.set(type, { image: context.canvas, w, h, mm: spanMm, projection: 1,
    axis: [[0, .5], [1, .5]], anchor: [.5, .5], mask: context.getImageData(0, 0, w, h) });
}

function rotate(at: Point, angle: number): Point {
  return {
    x: at.x * Math.cos(angle) - at.y * Math.sin(angle),
    y: at.x * Math.sin(angle) + at.y * Math.cos(angle),
  };
}

export function spritePose(art: ComposedArt, style: SceneryStyle, at: Point, parentAngle = 0): SpritePose {
  const image = art.sprites.get(style.type);
  if (!image) throw new Error('Missing calibrated sprite ' + style.type);
  const { axis: [a, b], mm, projection, w, h } = image;
  const flat = style.flat ?? 1;
  const length = Math.hypot((b[0] - a[0]) * w, (b[1] - a[1]) * h * flat);
  return {
    ...at, art: image, flat, scale: composedPixelsPerMm * (style.mm ?? mm) * projection / length,
    flip: style.flip ? -1 : 1, angle: parentAngle + (style.angle ?? 0),
  };
}

export function spritePoint(pose: SpritePose, [u, v]: readonly number[]): Point {
  const { art, scale, angle, flip, x, y, flat } = pose;
  const offset = rotate({ x: (u - art.anchor[0]) * art.w * scale * flip, y: (v - art.anchor[1]) * art.h * scale * flat }, angle);
  return { x: x + offset.x, y: y + offset.y };
}

export function spriteLocal(pose: SpritePose, at: Point): Point {
  const { art, scale, angle, flip, x, y, flat } = pose;
  const delta = rotate({ x: at.x - x, y: at.y - y }, -angle);
  return {
    x: art.anchor[0] + delta.x / (art.w * scale * flip),
    y: art.anchor[1] + delta.y / (art.h * scale * flat),
  };
}

export function hitsSprite(pose: SpritePose, style: SceneryStyle, at: Point) {
  const { x, y } = spriteLocal(pose, at);
  const { art } = pose;
  if (style.rootCut && y > art.anchor[1] + 1 / art.h) return false;
  const u = Math.floor(x * art.mask.width);
  const v = Math.floor(y * art.mask.height);
  return u >= 0 && v >= 0 && u < art.mask.width && v < art.mask.height
    && art.mask.data[(v * art.mask.width + u) * 4 + 3] >= 96;
}

export function drawShadow(context: CanvasRenderingContext2D, at: Point, [w, h, strength]: readonly number[]) {
  context.save();
  context.translate(at.x + 3, at.y - 1);
  context.scale(w / 2, h / 2);
  const fade = context.createRadialGradient(0, 0, .05, 0, 0, 1);
  fade.addColorStop(0, 'rgba(18,26,21,' + strength + ')');
  fade.addColorStop(.54, 'rgba(18,26,21,' + strength * .55 + ')');
  fade.addColorStop(1, '#121a1500');
  context.fillStyle = fade;
  context.fillRect(-1, -1, 2, 2);
  context.restore();
}

export function drawScenerySprite(context: CanvasRenderingContext2D, style: SceneryStyle, pose: SpritePose, mask = false) {
  const { art, scale, x, y, angle, flip, flat } = pose;
  context.save();
  context.translate(x, y);
  context.rotate(angle);
  context.scale(scale * flip, scale * flat);
  if (!mask) {
    context.globalAlpha *= style.opacity ?? 1;
    if (style.shade !== undefined || style.saturation !== undefined) {
      context.filter = 'brightness(' + (style.shade ?? 1) + ') saturate(' + (style.saturation ?? 1) + ')';
    }
  }
  const left = -art.anchor[0] * art.w;
  const top = -art.anchor[1] * art.h;
  if (style.rootCut) {
    const exposed = Math.min(art.h, art.anchor[1] * art.h + 1);
    context.drawImage(art.image, 0, 0, art.w, exposed, left, top, art.w, exposed);
  } else {
    context.drawImage(art.image, left, top);
  }
  context.restore();
}

export function drawFloatRings(context: CanvasRenderingContext2D, pose: SpritePose, sun: number) {
  const { art } = pose;
  const [a, b] = art.axis;
  const span = Math.hypot((b[0] - a[0]) * art.w, (b[1] - a[1]) * art.h);
  context.save();
  context.translate(pose.x, pose.y - 5);
  context.strokeStyle = '#d7edd0';
  context.lineWidth = .9;
  for (let index = 0; index < 3; index++) {
    context.globalAlpha = (.18 + .39 * sun) * (1 - index * .26);
    const radius = (art.mm * composedPixelsPerMm * .48 + index * 4) * (pose.scale / (composedPixelsPerMm * art.mm / span));
    context.beginPath();
    context.ellipse(0, 0, radius, 5.5 + index * 1.8, 0, Math.PI * 1.05, Math.PI * (1.55 + index * .06));
    context.stroke();
    context.beginPath();
    context.ellipse(0, 0, radius, 5.5 + index * 1.8, 0, 0, Math.PI * (.52 + index * .05));
    context.stroke();
  }
  context.restore();
}

export function spriteBounds(pose: SpritePose, style: SceneryStyle): Rect {
  const edge = style.rootCut ? Math.min(1, pose.art.anchor[1] + 1 / pose.art.h) : 1;
  const corners = [[0, 0], [1, 0], [0, edge], [1, edge]].map((corner) => spritePoint(pose, corner));
  const x = Math.min(...corners.map((point) => point.x));
  const y = Math.min(...corners.map((point) => point.y));
  return {
    x, y, width: Math.max(...corners.map((point) => point.x)) - x,
    height: Math.max(...corners.map((point) => point.y)) - y,
  };
}
