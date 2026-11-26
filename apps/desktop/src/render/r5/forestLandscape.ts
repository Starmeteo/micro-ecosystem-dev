import type { WorldSnapshot } from '../../world';
import baseUrl from '../assets/r5/forest-landscape/forest-base.png';
import woodUrl from '../assets/r5/forest-landscape/forest-wood.png';
import stoneUrl from '../assets/r5/forest-landscape/forest-stone.png';
import mossUrl from '../assets/r5/forest-landscape/forest-moss.png';
import fernUrl from '../assets/r5/forest-landscape/forest-fern.png';
import leafUrl from '../assets/r5/forest-landscape/forest-leaf.png';
import { R5_HEIGHT, R5_WIDTH, clamp } from './geometry';
import { forestArt } from './forestPlacement';
import type { SampleImages } from './sceneArt';

export type ForestLandscape = {
  base: HTMLImageElement;
  fern: HTMLImageElement;
  moss: HTMLImageElement;
  materials: SampleImages;
};

type MaterialRegion = {
  crop: readonly [x: number, y: number, width: number, height: number];
  sourceRise?: number;
  target?: readonly [top: number, height: number, rise: number];
};

const stoneRegion: MaterialRegion = { crop: [24, 11, 378, 255] };
const smallStoneRegion: MaterialRegion = { crop: [94, 65, 236, 151] };
const woodRegion: MaterialRegion = { crop: [150, 248, 1390, 105], sourceRise: -168, target: [64, 42, -59] };
const barkRegion: MaterialRegion = { crop: [448, 225, 816, 126], sourceRise: -114 };
const mossRegion: MaterialRegion = { crop: [93, 92, 329, 41] };
const leafRegion: MaterialRegion = { crop: [31, 64, 341, 100], sourceRise: -32 };
let cached: Promise<ForestLandscape> | undefined;

async function decode(url: string) {
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

async function applyMaterial(original: HTMLImageElement, material: HTMLImageElement, region: MaterialRegion, strength: number) {
  const { naturalWidth: width, naturalHeight: height } = original;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('Unable to compose forest landscape layers');
  const [x, y, sampleWidth, sampleHeight] = region.crop;
  const [top, materialHeight, rise] = region.target ?? [0, height, 0];
  const scaleX = width / sampleWidth;
  const scaleY = materialHeight / sampleHeight;
  const shear = (rise - (region.sourceRise ?? 0) * scaleY) / sampleWidth;
  context.imageSmoothingQuality = 'high';
  context.setTransform(scaleX, shear, 0, scaleY, -scaleX * x, top - shear * x - scaleY * y);
  context.drawImage(material, 0, 0);
  const texture = context.getImageData(0, 0, width, height).data;

  context.resetTransform();
  context.clearRect(0, 0, width, height);
  context.drawImage(original, 0, 0);
  const layer = context.getImageData(0, 0, width, height);
  for (let pixel = 0; pixel < layer.data.length; pixel += 4) {
    if (!layer.data[pixel + 3]) continue;
    const blend = strength * texture[pixel + 3] / 255;
    for (let channel = pixel; channel < pixel + 3; channel++) {
      layer.data[channel] += (texture[channel] - layer.data[channel]) * blend;
    }
  }
  // Keep every contact, cut-out edge and transparent gap from the existing layered cel.
  context.putImageData(layer, 0, 0);
  return decode(canvas.toDataURL());
}

export function loadForestLandscape(images: SampleImages) {
  cached ??= (async () => {
    const [base, fern, moss, wood, stone, leaf] = await Promise.all([
      baseUrl, fernUrl, mossUrl, woodUrl, stoneUrl, leafUrl,
    ].map(decode));
    const { objects, stonework, barkShelters, leafPatches } = forestArt;
    const materials = new Map(images);
    async function finish(path: string | undefined, source: HTMLImageElement, region: MaterialRegion, strength: number) {
      if (!path) return;
      const original = images.get(path);
      if (!original) throw new Error(`Missing forest layer: ${path}`);
      materials.set(path, await applyMaterial(original, source, region, strength));
    }
    await Promise.all([
      finish(objects.rock.sprites.default, stone, stoneRegion, .87),
      ...Object.values(stonework ?? {}).map(({ sprites }) => finish(sprites.default, stone, smallStoneRegion, .81)),
      finish(objects.log.sprites.back, wood, woodRegion, .82),
      finish(objects.log.sprites.front, wood, woodRegion, .77),
      finish(objects.log.sprites.growth, moss, mossRegion, .70),
      ...Object.values(barkShelters ?? {}).flatMap(({ sprites }) => [
        finish(sprites.back, wood, barkRegion, .79), finish(sprites.front, wood, barkRegion, .74),
      ]),
      ...Object.values(leafPatches ?? {}).flatMap(({ sprites }) => [
        finish(sprites.default, leaf, leafRegion, .78), finish(sprites.worn, leaf, leafRegion, .54),
        finish(sprites.trace, leaf, leafRegion, .39),
      ]),
    ]);
    return { base, fern, moss, materials };
  })().catch((error: unknown) => {
    cached = undefined;
    throw error;
  });
  return cached;
}

export function paintForestLandscape(context: CanvasRenderingContext2D, art: ForestLandscape, snapshot: WorldSnapshot) {
  context.save();
  context.imageSmoothingEnabled = true;
  context.imageSmoothingQuality = 'high';
  context.drawImage(art.base, 0, 0, R5_WIDTH, R5_HEIGHT);

  // These fixed scenic patches depict the existing vegetation readings for each side.
  const life = snapshot.forest?.life;
  for (const side of [0, 1] as const) {
    const fernCarbon = life?.fernMgC[side] ?? 90;
    const mossCarbon = life?.mossMgC[side] ?? 192;
    context.globalAlpha = Math.sqrt(clamp(fernCarbon / 90, 0, 1));
    const fernWidth = 121;
    const fernHeight = fernWidth * art.fern.naturalHeight / art.fern.naturalWidth;
    context.drawImage(art.fern, side === 0 ? -31 : 394, (side === 0 ? 145 : 158) - fernHeight, fernWidth, fernHeight);
    context.globalAlpha = Math.sqrt(clamp(mossCarbon / 192, 0, 1)) * .88;
    const mossWidth = side === 0 ? 72 : 88;
    const mossHeight = mossWidth * art.moss.naturalHeight / art.moss.naturalWidth;
    context.drawImage(art.moss, side === 0 ? 20 : 260, (side === 0 ? 161 : 160) - mossHeight, mossWidth, mossHeight);
  }
  context.restore();
}
