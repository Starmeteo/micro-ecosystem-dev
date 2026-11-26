import type { Point } from './geometry';

export function drawAnimalMarker(context: CanvasRenderingContext2D, point: Point) {
  context.save();
  context.strokeStyle = 'rgba(14, 28, 29, .85)';
  context.lineWidth = 2.5;
  context.beginPath();
  context.ellipse(point.x, point.y, 12, 7, 0, 0, 2 * Math.PI);
  context.stroke();
  context.strokeStyle = '#bfdfd5';
  context.lineWidth = 1;
  context.stroke();
  context.restore();
}
