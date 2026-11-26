import dryUrl from '../assets/r5/pond-landscape/dry-basin.png';
import ambientUrl from '../assets/r5/pond-landscape/water-ambient.png';
import directUrl from '../assets/r5/pond-landscape/water-direct.png';
import leafUrl from '../assets/r5/pond-landscape/pond-leaf.png';
import hornwortUrl from '../assets/r5/pond-landscape/hornwort.png';
import { R5_HEIGHT, R5_WIDTH, clamp } from './geometry';
import { bedY } from './pondStoneGeometry';

const imageSources = { dry: dryUrl, ambient: ambientUrl, direct: directUrl, leaf: leafUrl, hornwort: hornwortUrl };
export type PondLandscapeArt = Record<keyof typeof imageSources, HTMLImageElement>;
let decoded: Promise<PondLandscapeArt> | undefined;

export function loadPondLandscapeArt() {
  decoded ??= Promise.all(Object.entries(imageSources).map(async ([key, src]) => {
    const image = new Image();
    image.src = src;
    await image.decode();
    return [key, image] as const;
  })).then((entries) => Object.fromEntries(entries) as PondLandscapeArt).catch((error: unknown) => {
    decoded = undefined;
    throw error;
  });
  return decoded;
}

function fade(from: number, to: number, amount: number) {
  return from + (to - from) * amount;
}

function smooth(from: number, to: number, value: number) {
  const t = clamp((value - from) / (to - from), 0, 1);
  return t * t * (3 - 2 * t);
}

export function pondLandscapeLight(phase: number, seasonal: readonly [number, number, number]) {
  const elevation = Math.cos((phase - .5) * Math.PI * 2);
  const ambient = smooth(-.28, .6, elevation);
  const direct = smooth(-.08, .55, elevation);
  const twilight = (1 - smooth(.04, .55, Math.abs(elevation - .12))) * (phase < .5 ? .37 : .7);
  const night = [78, 105, 148];
  const horizon = phase < .5 ? [173, 192, 219] : [235, 168, 154];
  const tint = seasonal.map((channel, index) => Math.round(
    fade(fade(night[index], channel, ambient), horizon[index], twilight),
  ));
  return { direct, tint: `rgb(${tint.join(', ')})` };
}

export function drawPondLandscapeWater(
  context: CanvasRenderingContext2D, art: PondLandscapeArt, surface: number, direct: number,
) {
  // The first band of the texture is the water surface. Shallow water samples
  // progressively more of the sediment, while the actual clip comes from the basin.
  const depth = bedY - surface;
  const crop = .58 * (1 - smooth(8, 74, depth));
  const height = R5_HEIGHT - surface;
  const paint = (image: HTMLImageElement) => {
    const start = image.naturalHeight * crop;
    context.drawImage(image, 0, start, image.naturalWidth, image.naturalHeight - start,
      0, surface, R5_WIDTH, height);
  };
  paint(art.ambient);
  context.globalAlpha = direct;
  if (direct > 0) paint(art.direct);
  context.globalAlpha = 1;
}

const highlights = [72, 109, 141, 179, 224, 262, 308, 348, 395, 423] as const;

export function drawPondLandscapeSurface(
  context: CanvasRenderingContext2D, surface: number, seconds: number, direct: number,
) {
  const depth = bedY - surface;
  context.fillStyle = direct > .26 ? '#dfd7a0' : '#a4c2cb';
  for (const [index, anchor] of highlights.entries()) {
    const ripple = Math.sin(seconds * .9 + index * 1.75);
    const x = Math.round(anchor + ripple * 4);
    const y = Math.floor(surface + Math.min(depth - .5, (index * 7) % 5));
    const span = 3 + (index * 5) % 11;
    context.globalAlpha = (.09 + direct * .27) * (.74 + ripple * .26);
    context.fillRect(x, y, span, 1);
    context.fillRect(x + 2, y + 1, Math.ceil(span * .37), 1);
  }
  context.globalAlpha = 1;
}
