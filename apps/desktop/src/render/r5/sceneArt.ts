import manifestJson from '../assets/r5/manifest.json';
import type { BarkShelterPreset, LeafPatchPreset, StoneworkPreset } from '../../world';
import { R5_HEIGHT, R5_WIDTH, type Point, type Rect, type R5SampleObject, type R5SampleTheme, clamp, point, sampleProjection } from './geometry';

type Pair = readonly [number, number];
type Axis = readonly [Pair, Pair];

type SpriteName =
  | 'back' | 'front' | 'growth' | 'shadow' | 'default' | 'walk' | 'hidden' | 'swim' | 'interior' | 'preview'
  | 'worn' | 'trace' | 'depleted';

export type ArtObject = {
  width: number;
  height: number;
  anchor: Pair;
  position: Pair;
  lengthMm: number;
  body: Axis;
  contact: Pair[];
  perch?: Pair;
  cover?: Pair;
  sprites: Partial<Record<SpriteName, string>>;
};

export type StoneworkArt = ArtObject & {
  placementRange: Pair;
  parts: { offsetMm: number; lengthMm: number; heightPx: number }[];
};

export type BarkShelterArt = ArtObject & { placementRange: Pair; footOffsetMm: number };
export type LeafPatchArt = ArtObject & { placementRange: Pair };

export type SceneArt = {
  referenceWidthMm: number;
  layers: { base: string; foreground: string; light?: string; surface?: string };
  objects: {
    rock: ArtObject;
    log: ArtObject;
    leaf?: ArtObject;
    leafLarge?: ArtObject;
    moss?: ArtObject;
    plant?: ArtObject;
    woodlouse?: ArtObject;
    shrimp?: ArtObject;
  };
  stonework?: Record<StoneworkPreset, StoneworkArt>;
  barkShelters?: Record<BarkShelterPreset, BarkShelterArt>;
  leafPatches?: Record<LeafPatchPreset, LeafPatchArt>;
  waterline?: number;
  waterBounds?: readonly [number, number, number, number];
};

export type SampleImages = ReadonlyMap<string, HTMLImageElement>;
export type SamplePlacements = Record<R5SampleObject, Point>;
export type SamplePose = 0 | 1;

const manifest = manifestJson as unknown as Record<R5SampleTheme, SceneArt>;
const pngUrls = import.meta.glob<string>('../assets/r5/*/*.png', { eager: true, query: '?url', import: 'default' });
const loaded = new Map<R5SampleTheme, Promise<SampleImages>>();
const hitMasks = new WeakMap<HTMLImageElement, Uint8ClampedArray>();

export function sceneArt(theme: R5SampleTheme) {
  return manifest[theme];
}

export function loadSampleArt(theme: R5SampleTheme): Promise<SampleImages> {
  const cached = loaded.get(theme);
  if (cached) return cached;

  const art = sceneArt(theme);
  const paths = new Map<string, { width: number; height: number }>();
  for (const path of Object.values(art.layers)) paths.set(path, { width: R5_WIDTH, height: R5_HEIGHT });
  for (const object of [
    ...Object.values(art.objects), ...Object.values(art.stonework ?? {}), ...Object.values(art.barkShelters ?? {}),
    ...Object.values(art.leafPatches ?? {}),
  ]) {
    for (const path of Object.values(object.sprites)) paths.set(path, object);
  }
  const images = Promise.all(Array.from(paths, async ([path, size]) => {
    const url = pngUrls[`../assets/r5/${path}`];
    if (!url) throw new Error(`R5 asset not found: ${path}`);
    const image = new Image();
    image.src = url;
    await image.decode();
    if (image.naturalWidth !== size.width || image.naturalHeight !== size.height) {
      throw new Error(`R5 asset dimensions do not match the manifest: ${path}`);
    }
    return [path, image] as const;
  })).then((entries) => new Map(entries)).catch((error: unknown) => {
    loaded.delete(theme);
    throw error;
  });
  loaded.set(theme, images);
  return images;
}

export function initialPlacements(art: SceneArt): SamplePlacements {
  return { stone: point(art.objects.rock.position), log: point(art.objects.log.position) };
}

export function placedArt(art: SceneArt, object: R5SampleObject) {
  return object === 'stone' ? art.objects.rock : art.objects.log;
}

export function localPoint(object: ArtObject, local: Pair, position: Point, scale = 1): Point {
  return {
    x: position.x + (local[0] - object.anchor[0]) * scale,
    y: position.y + (local[1] - object.anchor[1]) * scale,
  };
}

export function hitSprite(image: HTMLImageElement, x: number, y: number, tolerance = 0) {
  let pixels = hitMasks.get(image);
  const { naturalWidth: width, naturalHeight: height } = image;
  if (!pixels) {
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return false;
    ctx.drawImage(image, 0, 0);
    pixels = ctx.getImageData(0, 0, width, height).data;
    hitMasks.set(image, pixels);
  }
  for (let row = Math.max(0, y - tolerance); row <= Math.min(height - 1, y + tolerance); row++) {
    for (let col = Math.max(0, x - tolerance); col <= Math.min(width - 1, x + tolerance); col++) {
      if (pixels[(row * width + col) * 4 + 3] > 48) return true;
    }
  }
  return false;
}

export function placementHit(
  object: ArtObject, position: Point, at: Point, images: SampleImages, mirrored = false, tolerance = 3,
) {
  const origin = localPoint(object, [0, 0], position);
  const x = Math.floor((mirrored ? position.x * 2 - at.x : at.x) - origin.x);
  const y = Math.floor(at.y - origin.y);
  if (x < -tolerance || y < -tolerance || x >= object.width + tolerance || y >= object.height + tolerance) return false;
  const { back, front, growth, default: single } = object.sprites;
  return [back, front, single, growth].some((path) => {
    const image = path ? images.get(path) : undefined;
    return image ? hitSprite(image, x, y, tolerance) : false;
  });
}

export function drawArtAt(
  context: CanvasRenderingContext2D, images: SampleImages,
  object: ArtObject, position: Point, path?: string, scale = 1, mirrored = false,
) {
  const sprite = path ? images.get(path) : undefined;
  if (!sprite) return;
  context.save();
  context.translate(position.x, position.y);
  context.scale(mirrored ? -scale : scale, scale);
  context.drawImage(sprite, -object.anchor[0], -object.anchor[1], object.width, object.height);
  context.restore();
}

export function supportPoint(art: SceneArt, placements: SamplePlacements) {
  const log = art.objects.log;
  const contact = log.cover ?? log.perch;
  return contact ? localPoint(log, contact, placements.log) : placements.log;
}

export function sampleAnimal(art: SceneArt, theme: R5SampleTheme, placements: SamplePlacements, pose: SamplePose) {
  const animal = theme === 'forest' ? art.objects.woodlouse! : art.objects.shrimp!;
  const log = art.objects.log;
  const [start, end] = animal.body;
  const axis = Math.hypot(end[0] - start[0], end[1] - start[1]);
  const scale = sampleProjection(animal.lengthMm, art.referenceWidthMm, R5_WIDTH) / axis;
  if (theme === 'forest' && pose === 1 && log.cover && animal.sprites.hidden) {
    return { animal, scale, position: localPoint(log, log.cover, placements.log), sprite: animal.sprites.hidden, attached: true };
  }
  if (theme === 'pond' && pose === 0) {
    return { animal, scale, position: supportPoint(art, placements), sprite: animal.sprites.default, attached: true };
  }
  return {
    animal,
    scale,
    position: point(animal.position),
    sprite: (theme === 'pond' ? animal.sprites.swim : animal.sprites.walk) ?? animal.sprites.default,
    attached: false,
  };
}

export function waterWindow(art: SceneArt, level: number): Rect | null {
  if (!art.waterBounds || art.waterline === undefined) return null;
  const [x, y, width, height] = art.waterBounds;
  // The comparison moves the surface within the illustrated wet range.
  const top = Math.max(y, art.waterline + Math.round((1 - clamp(level, 0, 1)) * 60));
  return { x, y: top, width, height: Math.max(0, y + height - top) };
}
