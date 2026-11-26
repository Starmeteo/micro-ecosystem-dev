import type { ForestSubstrateCell } from '../../world';
import { forestPixelsPerMm, hitsForestSprite, type ForestPose } from './forestComposedArt';
import { forestSoilIndex, type ForestObject } from './forestPlacement';

export interface ForestMyceliumObservation {
  cellIndex: number;
  decomposerMgC: number;
  spanMm: 0 | 5 | 15 | 30;
}

export function observeForestMycelium(cells: readonly ForestSubstrateCell[] | undefined,
  wood: Pick<ForestObject, 'kind' | 'preset' | 'position'>): ForestMyceliumObservation | undefined {
  const weatheredBark = wood.kind === 'barkShelter' && (wood.preset === 'arch' || wood.preset === 'split');
  if (wood.kind !== 'log' && !weatheredBark) return;
  const index = forestSoilIndex(wood.position);
  const source = cells?.find(({ cellIndex }) => cellIndex === index);
  const decomposerMgC = source?.decomposerMgC;
  if (!source || decomposerMgC === undefined) return;
  // The three coverage tiers belong to the illustration, not to organism identification or growth.
  const spanMm = decomposerMgC <= 0 ? 0 : decomposerMgC < .8 ? 5 : decomposerMgC < 3.2 ? 15 : 30;
  return { cellIndex: source.cellIndex, decomposerMgC, spanMm };
}

// Local clusters follow a stepped splinter line, branching into sparse threads and pale flecks.
const paleFleck = [
  [6, 4, 2], [9, 4, 4], [14, 3, 2], [16, 4, 4], [20, 3, 3],
  [8, 5, 5], [13, 5, 4], [19, 5, 2], [10, 6, 3],
] as const;
const threads = [
  [0, 8, 2], [2, 7, 2], [4, 6, 2], [6, 5, 5], [11, 4, 4], [15, 3, 5], [20, 2, 3], [23, 1, 1],
  [5, 7, 2], [6, 8, 2], [8, 9, 2], [10, 10, 2], [12, 11, 2],
  [9, 3, 3], [8, 2, 2], [6, 1, 2], [4, 0, 2],
  [13, 6, 3], [16, 7, 2], [18, 8, 1], [20, 4, 2], [22, 5, 1],
] as const;
const anchors = [[.48, .58], [.39, .54], [.58, .46], [.33, .63], [.62, .66], [.47, .41]] as const;

export function paintForestMycelium(ctx: CanvasRenderingContext2D, pose: ForestPose, sample: ForestMyceliumObservation) {
  if (!sample.spanMm) return;
  const span = sample.spanMm * forestPixelsPerMm;
  const unit = span / 24;
  const thickness = Math.min(unit, .48 * forestPixelsPerMm);
  const cos = Math.cos(pose.angle);
  const sin = Math.sin(pose.angle);
  const onSurface = (x: number, y: number) => hitsForestSprite({
    x: pose.at.x + x * pose.flip * cos - y * sin,
    y: pose.at.y + x * pose.flip * sin + y * cos,
  }, pose);
  const center = anchors.map(([u, v]) => ({ x: (u - pose.fx) * pose.width, y: (v - pose.fy) * pose.height }))
    .find(({ x, y }) => onSurface(x, y) && onSurface(x - span * .4, y) && onSurface(x + span * .4, y));
  if (!center) return;

  ctx.save();
  ctx.translate(pose.at.x, pose.at.y);
  ctx.rotate(pose.angle);
  ctx.scale(pose.flip, 1);
  ctx.globalAlpha *= .30 + .36 * Math.min(1, sample.decomposerMgC / 3.2);
  const paint = (strand: readonly (readonly [number, number, number])[], color: string) => {
    ctx.fillStyle = color;
    for (const [column, row, length] of strand) {
      const top = center.y + (row - 5) * unit * .77;
      for (let offset = 0; offset < length; offset++) {
        const left = center.x + (column + offset) * unit - span / 2;
        if ([left, left + unit].every((x) => onSurface(x, top) && onSurface(x, top + thickness))) {
          ctx.fillRect(left, top, unit, thickness);
        }
      }
    }
  };
  paint(paleFleck, '#bcb99b');
  paint(threads, '#e4dcc2');
  ctx.restore();
}
