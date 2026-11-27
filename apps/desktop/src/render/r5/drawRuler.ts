import { R5_WIDTH, type Rect, sampleProjection } from './geometry';

export function drawRuler(
  ctx: CanvasRenderingContext2D, viewport: Rect, output: HTMLCanvasElement,
  referenceWidthMm: number, dpr: number,
) {
  ctx.save();
  ctx.lineWidth = Math.max(1, dpr);
  const mm = 50;
  const left = 14 * dpr;
  const bottom = output.height - 14 * dpr;
  const ruler = sampleProjection(mm, referenceWidthMm, output.width, R5_WIDTH / viewport.width);
  const padX = 8 * dpr;
  const boxX = left - padX;
  const boxY = bottom - 23 * dpr;
  const boxW = ruler + padX * 2;
  const boxH = 31 * dpr;
  const radius = 7 * dpr;

  ctx.fillStyle = 'rgba(12, 22, 26, 0.76)';
  ctx.strokeStyle = 'rgba(220, 246, 238, 0.28)';
  ctx.beginPath();
  ctx.roundRect(boxX, boxY, boxW, boxH, radius);
  ctx.fill();
  ctx.stroke();

  ctx.strokeStyle = '#ebfaf4';
  ctx.fillStyle = '#ebfaf4';
  ctx.font = `600 ${10.5 * dpr}px "Segoe UI Variable Text", "Inter", system-ui, sans-serif`;
  ctx.textAlign = 'center';
  ctx.fillText(`${mm} mm`, left + ruler / 2, bottom - 7 * dpr);
  ctx.beginPath();
  ctx.moveTo(left, bottom - 3.5 * dpr);
  ctx.lineTo(left, bottom + 2 * dpr);
  ctx.lineTo(left + ruler, bottom + 2 * dpr);
  ctx.lineTo(left + ruler, bottom - 3.5 * dpr);
  ctx.moveTo(left + ruler / 2, bottom - 1 * dpr);
  ctx.lineTo(left + ruler / 2, bottom + 2 * dpr);
  ctx.stroke();
  ctx.restore();
}
