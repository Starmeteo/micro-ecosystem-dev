import type { PondBasinSnapshot, PondMossParent, PondMossPatch, PondMossPreset } from '../../world';
import art from '../assets/r5/placement-geometry.json';
import type { Point } from './geometry';
import { bedY, bedPoint, pondMossSupport, pondTerrainObjects, stoneWaterHeight } from './pondStoneGeometry';

const specs = art.pond.attachedMoss;
type MossFrame = 'full' | 'sparse' | 'exposed';
export type PondMossDraft = { preset: PondMossPreset; parent: PondMossParent; offset: number; id?: number };
type MossVisual = { preset: PondMossPreset; point: Point; frame: MossFrame; id?: number };
type MossDraftSite = 'ground' | 'occupied' | 'out-of-ground' | 'exposed';
export type PondMossPreview = { site: MossDraftSite; visual?: MossVisual };

function mossFrame(patch: PondMossPatch): MossFrame {
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
