import { clamp, R5_HEIGHT, R5_WIDTH, type Rect } from './geometry';
import { bedLeft, bedWidth, bedY } from './pondStoneGeometry';

const fullFrame: Rect = { x: 0, y: 0, width: R5_WIDTH, height: R5_HEIGHT };
const shallowDepth = 72;
const closeupDepth = 116;
const airMargin = 8;
const sedimentMargin = 14;

export function pondUnderwaterViewport(surfaceY: number): Rect {
  const depth = bedY - surfaceY;
  if (!Number.isFinite(surfaceY) || surfaceY < 0 || depth <= shallowDepth) return fullFrame;

  const bottom = Math.min(bedY + sedimentMargin, R5_HEIGHT);
  const top = Math.max(0, surfaceY - airMargin);
  const height = Math.max(R5_HEIGHT / 2, bottom - top);
  const width = height * R5_WIDTH / R5_HEIGHT;
  const left = clamp(bedLeft + (bedWidth - width) / 2, 0, R5_WIDTH - width);
  // Blend the whole frame back in as the water gets shallow. Both views stay 16:9.
  const closeup = clamp((depth - shallowDepth) / (closeupDepth - shallowDepth), 0, 1);
  return {
    x: left * closeup,
    y: (bottom - height) * closeup,
    width: R5_WIDTH + (width - R5_WIDTH) * closeup,
    height: R5_HEIGHT + (height - R5_HEIGHT) * closeup,
  };
}
