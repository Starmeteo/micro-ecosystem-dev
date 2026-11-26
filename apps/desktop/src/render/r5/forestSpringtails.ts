import { forestHeight, forestPixelsPerMm, forestWidth } from './forestComposedArt';

export type ForestSpringtailSide = 'left' | 'right';
export type ForestSpringtailView = 'both' | ForestSpringtailSide;
export type ForestSpringtailStock = readonly [number, number];

// These observation points are deliberately fixed examples on the floor.
// A point is one density mark, without a simulated identity or exact location.
const groundMarks = [
  [
    [.38, .84, -.31], [.40, .88, .26], [.35, .87, 1.29], [.43, .79, -.69], [.32, .89, .48],
    [.45, .84, .16], [.38, .76, .98], [.29, .91, -.55], [.46, .91, .76],
  ],
  [
    [.63, .85, -.63], [.69, .84, .22], [.61, .89, 1.23], [.67, .91, -.81], [.70, .88, .65],
    [.56, .81, .46], [.74, .79, -.22], [.58, .91, 1.02], [.80, .90, -.44],
  ],
] as const;

const bodyPixels = 1.5 * forestPixelsPerMm;
const stockPerMarkMgC = 2.5;

export function forestSpringtailMarkCounts(stock: ForestSpringtailStock) {
  return [0, 1].map(side => Math.min(groundMarks[side].length, Math.ceil(Math.max(0, stock[side]) / stockPerMarkMgC)));
}

export function forestSpringtailFocus(side: ForestSpringtailSide) {
  return { x: forestWidth * (side === 'left' ? .39 : .66), y: forestHeight * .85 };
}

export function paintForestSpringtails(context: CanvasRenderingContext2D, stock: ForestSpringtailStock) {
  const counts = forestSpringtailMarkCounts(stock);
  context.save();
  context.fillStyle = '#dfd9c9b0';
  for (const [side, amount] of counts.entries()) {
    for (const [x, y, angle] of groundMarks[side].slice(0, amount)) {
      context.beginPath();
      context.ellipse(x * forestWidth, y * forestHeight, bodyPixels / 2, bodyPixels * .20, angle, 0, Math.PI * 2);
      context.fill();
    }
  }
  context.restore();
}
