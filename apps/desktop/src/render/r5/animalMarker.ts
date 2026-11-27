import type { Point } from './geometry';

export function drawAnimalMarker(context: CanvasRenderingContext2D, point: Point) {
  context.save();
  context.fillStyle = 'rgba(176, 244, 222, 0.14)';
  context.beginPath();
  context.ellipse(point.x, point.y, 14, 8.5, 0, 0, 2 * Math.PI);
  context.fill();

  context.strokeStyle = 'rgba(10, 24, 26, 0.88)';
  context.lineWidth = 2.8;
  context.beginPath();
  context.ellipse(point.x, point.y, 12.5, 7.5, 0, 0, 2 * Math.PI);
  context.stroke();

  context.strokeStyle = '#c8f7e6';
  context.lineWidth = 1.25;
  context.stroke();
  context.restore();
}
