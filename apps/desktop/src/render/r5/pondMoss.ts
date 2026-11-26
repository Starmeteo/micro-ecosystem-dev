import type { PondBasinSnapshot, PondMossParent, PondMossPatch, PondMossPreset } from '../../world';
import art from '../assets/r5/manifest.json';
import type { Point } from './geometry';
import { bedY, bedPoint, pondMossSupport, pondTerrainObjects, stoneWaterHeight } from './pondStoneGeometry';
import { hitSprite } from './sceneArt';

const specs = art.pond.attachedMoss;
const urls = import.meta.glob<string>('../assets/r5/pond/attached-plant-*.png', { eager: true, query: '?url', import: 'default' });
type MossFrame = 'full' | 'sparse' | 'exposed';
export type PondMossArt = Record<PondMossPreset, Record<MossFrame, HTMLImageElement>>;
export type PondMossDraft = { preset: PondMossPreset; parent: PondMossParent; offset: number; id?: number };
type MossVisual = { preset: PondMossPreset; point: Point; frame: MossFrame; id?: number };
type MossDraftSite = 'ground' | 'occupied' | 'out-of-ground' | 'exposed';
export type PondMossPreview = { site: MossDraftSite; visual?: MossVisual };

let mossImages: Promise<PondMossArt> | undefined;
async function loadSpec(preset: PondMossPreset) {
  async function imageFor(frame: MossFrame) {
    const spec = specs[preset];
    const path = spec.sprites[frame];
    const url = urls[`../assets/r5/${path}`];
    if (!url) throw new Error(`Missing pond moss art: ${path}`);
    const image = new Image();
    image.src = url;
    await image.decode();
    if (image.naturalWidth !== spec.width || image.naturalHeight !== spec.height) {
      throw new Error(`Pond moss art dimensions do not match: ${path}`);
    }
    return image;
  }
  const [full, sparse, exposed] = await Promise.all([imageFor('full'), imageFor('sparse'), imageFor('exposed')]);
  return { full, sparse, exposed };
}

export function loadPondMossArt() {
  mossImages ??= Promise.all([loadSpec('mat'), loadSpec('frond')]).then(([mat, frond]) => ({ mat, frond }))
    .catch((error: unknown) => {
      mossImages = undefined;
      throw error;
    });
  return mossImages;
}

export function mossFrame(patch: PondMossPatch): MossFrame {
  if (!patch.submerged) return 'exposed';
  // The displayed stock chooses an authored painting density. The painting conveys no mass value.
  return patch.stockMgCnp[0] < specs[patch.preset].maxCarbonMg * .55 ? 'sparse' : 'full';
}

export function mossVisual(patch: PondMossPatch, offsetX = 0): MossVisual {
  return { preset: patch.preset, id: patch.id, frame: mossFrame(patch),
    point: { x: bedPoint(patch.x + offsetX).x, y: bedY - patch.surfaceHeightPx } };
}

export function sameMossParent(left: PondMossParent, right: PondMossParent) {
  return left.kind === right.kind && left.id === right.id;
}

export function previewPondMoss(pond: PondBasinSnapshot, draft?: PondMossDraft): PondMossPreview | undefined {
  if (!draft) return;
  const carrier = pondTerrainObjects(pond).find((item) => sameMossParent(item, draft.parent));
  const support = carrier ? pondMossSupport(carrier, draft.offset) : undefined;
  if (!carrier || !support) return { site: 'out-of-ground' };

  const current = pond.mossPatches?.find(({ id }) => id === draft.id);
  const point = { x: bedPoint(carrier.x).x + support.center + draft.offset * support.halfSpan, y: bedY - support.height };
  const submerged = stoneWaterHeight(pond) > support.height;
  const occupied = pond.mossPatches?.some((patch) => patch.id !== draft.id && sameMossParent(patch.parent, draft.parent));
  return {
    site: !submerged ? 'exposed' : occupied ? 'occupied' : 'ground',
    visual: {
      point, preset: draft.preset, id: draft.id,
      frame: !submerged ? 'exposed' : current ? mossFrame({ ...current, submerged: true }) : 'sparse',
    },
  };
}

export function drawPondMoss(context: CanvasRenderingContext2D, images: PondMossArt, visual: MossVisual) {
  const [x, y] = specs[visual.preset].anchor;
  context.drawImage(images[visual.preset][visual.frame], visual.point.x - x, visual.point.y - y);
}

export function markPondMoss(context: CanvasRenderingContext2D, visual: MossVisual, draftSite?: MossDraftSite) {
  const image = specs[visual.preset];
  const [x, y] = image.anchor;
  context.save();
  context.strokeStyle = draftSite === undefined ? '#b6eddb' : draftSite === 'ground' ? '#d3e9ea' : '#ffd39d';
  if (draftSite) context.setLineDash([3, 2]);
  context.strokeRect(visual.point.x - x + .5, visual.point.y - y + .5, image.width - 1, image.height - 1);
  context.restore();
}

export function pickPondMoss(pond: PondBasinSnapshot, at: Point, images: PondMossArt) {
  for (const patch of pond.mossPatches?.slice().reverse() ?? []) {
    const visual = mossVisual(patch);
    const spec = specs[patch.preset];
    const x = Math.floor(at.x - visual.point.x + spec.anchor[0]);
    const y = Math.floor(at.y - visual.point.y + spec.anchor[1]);
    if (x >= 0 && x < spec.width && y >= 0 && y < spec.height && hitSprite(images[patch.preset][visual.frame], x, y)) {
      return { id: patch.id, kind: 'plant' } as const;
    }
  }
}
