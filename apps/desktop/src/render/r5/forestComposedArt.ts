import definition from '../assets/r5/forest-composed/definition.json';
import type { Point } from './geometry';
import { clamp } from './geometry';
import { seasonColor } from '../seasonColor';

export const forestWidth = 576;
export const forestHeight = 324;
export const forestPixelsPerMm = forestWidth / 450;

interface Material {
  file: string;
  mm: number;
  axis: number[][];
  foot: number[];
}

export interface ForestStyle {
  id?: string;
  label?: string;
  type: string;
  depth: number;
  mm?: number;
  scale?: number;
  flat?: number;
  flip?: boolean;
  angle?: number;
  offsetX?: number;
  shade?: number;
  saturation?: number;
  opacity?: number;
  shadow?: number[];
  shadowAngle?: number;
  contact?: number[][];
  attachedOnly?: boolean;
  seedShellGroup?: string;
}

interface SourceRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface Sprite {
  image: HTMLImageElement;
  spec: Material;
  source: SourceRect;
  alpha: ImageData;
}

export interface ForestPose {
  at: Point;
  image: HTMLImageElement;
  alpha: ImageData;
  source: SourceRect;
  width: number;
  height: number;
  fx: number;
  fy: number;
  flip: number;
  angle: number;
}

export interface ComposedForestArt {
  bed: HTMLImageElement;
  sprites: Map<string, Sprite>;
}

export const forestSceneryStyles = new Map<string, ForestStyle>(definition.styles.map(style => [style.id, style]));
export const forestHeroStyles: Record<string, ForestStyle> = definition.physical;
const materials: Record<string, Material> = definition.art;
const pngPath = '../assets/r5/forest-composed/';
const urls = import.meta.glob<string>('../assets/r5/forest-composed/*.png', { eager: true, query: '?url', import: 'default' });

async function loadImage(file: string) {
  const url = urls[pngPath + file];
  if (!url) throw new Error('Missing composed forest asset: ' + file);
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

function sampleAlpha(image: HTMLImageElement) {
  const ratio = Math.min(1, 620 / Math.max(image.naturalWidth, image.naturalHeight));
  const canvas = document.createElement('canvas');
  canvas.width = Math.ceil(image.naturalWidth * ratio);
  canvas.height = Math.ceil(image.naturalHeight * ratio);
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('Composed forest requires Canvas 2D');
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  const alpha = context.getImageData(0, 0, canvas.width, canvas.height);
  let left = canvas.width;
  let top = canvas.height;
  let right = -1;
  let bottom = -1;
  for (let y = 0; y < canvas.height; y++) {
    for (let x = 0; x < canvas.width; x++) {
      if (alpha.data[(y * canvas.width + x) * 4 + 3] < 48) continue;
      left = Math.min(left, x);
      top = Math.min(top, y);
      right = Math.max(right, x);
      bottom = Math.max(bottom, y);
    }
  }
  if (right < left) throw new Error('Composed forest sprite has no visible pixels');
  const sx = image.naturalWidth / canvas.width;
  const sy = image.naturalHeight / canvas.height;
  return { alpha, source: { x: left * sx, y: top * sy, w: (right + 1 - left) * sx, h: (bottom + 1 - top) * sy } };
}

let cached: Promise<ComposedForestArt> | undefined;
export function loadComposedForestArt() {
  if (!cached) {
    const images = new Map<string, Promise<Omit<Sprite, 'spec'>>>();
    async function sprite(spec: Material): Promise<Sprite> {
      let decoded = images.get(spec.file);
      if (!decoded) {
        decoded = loadImage(spec.file).then(image => ({ image, ...sampleAlpha(image) }));
        images.set(spec.file, decoded);
      }
      return { ...await decoded, spec };
    }
    cached = Promise.all([loadImage(definition.bed), Promise.all(
      Object.entries(materials).map(async ([id, spec]) => [id, await sprite(spec)] as const),
    )]).then(([bed, entries]) => ({ bed, sprites: new Map(entries) })).catch((error: unknown) => {
      cached = undefined;
      throw error;
    });
  }
  return cached;
}

export function forestPose(art: ComposedForestArt, style: ForestStyle, anchor: Point, mirror = 1): ForestPose {
  const sprite = art.sprites.get(style.type);
  if (!sprite) throw new Error('Missing calibrated forest sprite: ' + style.type);
  const { source, spec, image, alpha } = sprite;
  const [[ax, ay], [bx, by]] = spec.axis;
  const flat = style.flat ?? 1;
  const axisLength = Math.hypot((bx - ax) * source.w, (by - ay) * source.h * flat);
  const scale = (style.mm ?? spec.mm) * forestPixelsPerMm / axisLength * (style.scale ?? 1);
  return {
    image, source, alpha, at: { x: anchor.x + (style.offsetX ?? 0) * mirror, y: anchor.y },
    width: source.w * scale, height: source.h * scale * flat, fx: spec.foot[0], fy: spec.foot[1],
    flip: (style.flip ? -1 : 1) * mirror, angle: (style.angle ?? 0) * mirror,
  };
}

export function hitsForestSprite(point: Point, pose: ForestPose) {
  const dx = point.x - pose.at.x;
  const dy = point.y - pose.at.y;
  const cos = Math.cos(pose.angle);
  const sin = Math.sin(pose.angle);
  const u = pose.fx + (dx * cos + dy * sin) / (pose.width * pose.flip);
  const v = pose.fy + (dy * cos - dx * sin) / pose.height;
  if (u < 0 || v < 0 || u >= 1 || v >= 1) return false;
  const { source, alpha, image } = pose;
  const x = Math.min(alpha.width - 1, Math.floor((source.x + u * source.w) * alpha.width / image.naturalWidth));
  const y = Math.min(alpha.height - 1, Math.floor((source.y + v * source.h) * alpha.height / image.naturalHeight));
  return alpha.data[(y * alpha.width + x) * 4 + 3] >= 102;
}

function contactShadow(ctx: CanvasRenderingContext2D, at: Point, mirror: number,
  [width, height, strength, offsetX = 0, offsetY = 0, angle = 0]: readonly number[]) {
  ctx.save();
  ctx.translate(at.x + offsetX * mirror, at.y + offsetY);
  ctx.rotate(angle * mirror);
  ctx.scale(width / 2, height / 2);
  const gradient = ctx.createRadialGradient(0, 0, .04, 0, 0, 1);
  gradient.addColorStop(0, 'rgba(13,15,12,' + strength + ')');
  gradient.addColorStop(.38, 'rgba(15,17,13,' + strength * .74 + ')');
  gradient.addColorStop(1, 'rgba(17,19,14,0)');
  ctx.fillStyle = gradient;
  ctx.fillRect(-1, -1, 2, 2);
  ctx.restore();
}

export function drawForestContact(ctx: CanvasRenderingContext2D, style: ForestStyle, pose: ForestPose, mirror: number) {
  if (style.shadow) {
    const [w, h, a, x = 0, y = 0] = style.shadow;
    contactShadow(ctx, pose.at, mirror, [w, h, a, x, y, style.shadowAngle ?? 0]);
  }
  for (const contact of style.contact ?? []) contactShadow(ctx, pose.at, mirror, contact);
}

export function drawForestSprite(ctx: CanvasRenderingContext2D, style: ForestStyle, pose: ForestPose,
  heightSlice: readonly [number, number] = [0, 1]) {
  const { source, at } = pose;
  ctx.save();
  ctx.translate(at.x, at.y);
  ctx.rotate(pose.angle);
  ctx.scale(pose.flip, 1);
  ctx.globalAlpha *= style.opacity ?? 1;
  if (style.shade !== undefined || style.saturation !== undefined) {
    ctx.filter = 'brightness(' + (style.shade ?? 1) + ') saturate(' + (style.saturation ?? 1) + ')';
  }
  const [top, bottom] = heightSlice;
  ctx.drawImage(pose.image, source.x, source.y + top * source.h, source.w, (bottom - top) * source.h,
    -pose.fx * pose.width, (top - pose.fy) * pose.height, pose.width, (bottom - top) * pose.height);
  ctx.restore();
}

export function paintComposedForestBed(ctx: CanvasRenderingContext2D, bed: HTMLImageElement) {
  const fit = Math.max(forestWidth / bed.naturalWidth, forestHeight / bed.naturalHeight);
  const w = bed.naturalWidth * fit;
  const h = bed.naturalHeight * fit;
  ctx.drawImage(bed, (forestWidth - w) / 2, (forestHeight - h) / 2, w, h);
}

function pool(ctx: CanvasRenderingContext2D, x: number, y: number, spread: number, squash: number, stops: [number, string][]) {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(1, squash);
  const glare = ctx.createRadialGradient(0, 0, 0, 0, 0, spread);
  for (const [position, color] of stops) glare.addColorStop(position, color);
  ctx.fillStyle = glare;
  ctx.fillRect(-spread, -spread, spread * 2, spread * 2);
  ctx.restore();
}

export function paintComposedForestLight(ctx: CanvasRenderingContext2D, phase: number, season?: number) {
  const sun = clamp((Math.cos((phase - .5) * Math.PI * 2) + .2) / .72, 0, 1);
  const half = sun < .5 ? sun * 2 : (sun - .5) * 2;
  const ease = half * half * (3 - 2 * half);
  const darkness = 1 - sun;
  const twilight = phase > .5 ? [214, 174, 133] : [173, 190, 205];
  const from = sun < .5 ? [82, 107, 145] : twilight;
  const to = sun < .5 ? twilight : seasonColor(season, 'light').map(channel => 255 - (255 - channel) * .29);
  ctx.save();
  ctx.globalCompositeOperation = 'multiply';
  ctx.fillStyle = 'rgb(' + from.map((channel, index) => Math.round(channel + (to[index] - channel) * ease)).join(',') + ')';
  ctx.fillRect(0, 0, forestWidth, forestHeight);
  ctx.globalCompositeOperation = 'screen';
  pool(ctx, 246, 219, 190, .61, [
    [0, 'rgba(208,157,68,' + sun ** 1.22 * .16 + ')'],
    [.37, 'rgba(187,135,60,' + sun ** 1.22 * .09 + ')'], [1, 'rgba(170,125,55,0)'],
  ]);
  pool(ctx, 479, 153, 117, .80, [
    [0, 'rgba(183,186,102,' + sun ** 1.3 * .16 + ')'],
    [.52, 'rgba(112,138,63,' + sun ** 1.3 * .10 + ')'], [1, 'rgba(119,133,64,0)'],
  ]);
  pool(ctx, 203, 73, 182, .68, [
    [0, 'rgba(125,159,94,' + sun * .22 + ')'],
    [.42, 'rgba(100,134,70,' + sun * .13 + ')'], [1, 'rgba(98,123,66,0)'],
  ]);
  pool(ctx, 340, 138, 240, .69, [
    [0, 'rgba(58,88,145,' + darkness * .24 + ')'],
    [.55, 'rgba(37,62,103,' + darkness * .13 + ')'], [1, 'rgba(16,35,86,0)'],
  ]);
  ctx.globalCompositeOperation = 'multiply';
  const edge = ctx.createRadialGradient(296, 194, 76, 282, 152, 370);
  edge.addColorStop(0, '#fffffff9');
  edge.addColorStop(.72, '#f8f9f3');
  edge.addColorStop(1, '#b7c3b3');
  ctx.fillStyle = edge;
  ctx.fillRect(0, 0, forestWidth, forestHeight);
  ctx.restore();
}

export function markForestSprite(ctx: CanvasRenderingContext2D, pose: ForestPose, color: string, draft: boolean) {
  const { at, width, height, fx, fy } = pose;
  ctx.save();
  ctx.translate(at.x, at.y);
  ctx.rotate(pose.angle);
  ctx.scale(pose.flip, 1);
  const left = -fx * width - 2;
  const top = -fy * height - 2;
  const right = left + width + 4;
  const bottom = top + height + 4;
  const corner = Math.min(8, width / 4, height / 4);
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.25;
  if (draft) ctx.setLineDash([3, 2]);
  ctx.beginPath();
  for (const [x, y, sx, sy] of [[left, top, 1, 1], [right, top, -1, 1], [right, bottom, -1, -1], [left, bottom, 1, -1]]) {
    ctx.moveTo(x, y + corner * sy);
    ctx.lineTo(x, y);
    ctx.lineTo(x + corner * sx, y);
  }
  ctx.stroke();
  ctx.restore();
}
