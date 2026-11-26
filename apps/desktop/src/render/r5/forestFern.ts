import type { ForestFernPatch } from '../../world';
import { forestPixelsPerMm, forestPose, type ComposedForestArt, type ForestStyle } from './forestComposedArt';
import { clamp, type Point } from './geometry';

type FernState = Pick<ForestFernPatch, 'stockMgCnp' | 'stressFraction'>;
type FernPreset = { initialMgCnp: readonly number[]; heightMm: number; widthMm: number };

export function fernFrame(patch: FernState, initialMgC: number) {
  if (patch.stressFraction >= .65) return 'stressed';
  const fraction = patch.stockMgCnp[0] / initialMgC;
  if (fraction < .34) return 'tender';
  if (fraction < .80) return 'small';
  return 'open';
}

export function fernPart(art: ComposedForestArt, patch: FernState, preset: FernPreset, anchor: Point) {
  const frame = fernFrame(patch, preset.initialMgCnp[0]);
  const abundance = clamp(Math.sqrt(patch.stockMgCnp[0] / preset.initialMgCnp[0]), .23, 180 / preset.heightMm);
  const stress = clamp(patch.stressFraction, 0, 1);
  const height = preset.heightMm * abundance * (1 - (.16 * stress));
  const width = preset.widthMm * abundance * (frame === 'tender' ? .65 : 1 - (.15 * stress));
  const depth = 3 + clamp((anchor.y - 221) / 15, 0, 3.4);
  const style: ForestStyle = { type: frame === 'open' ? 'fern' : 'fern-side', depth,
    angle: frame === 'stressed' ? .22 : 0, saturation: .91 - .69 * stress,
    shade: 1.02 - .24 * stress, shadow: [width * .46, Math.max(4, width * .12), .4] };
  const pose = forestPose(art, style, anchor);
  pose.width = width * forestPixelsPerMm;
  pose.height = height * forestPixelsPerMm / pose.fy;
  return { style, pose };
}
