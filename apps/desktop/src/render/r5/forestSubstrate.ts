import type { ForestSubstrateCell } from '../../world';
import { forestHeight, forestWidth } from './forestComposedArt';
import { forestSoilCell } from './forestPlacement';

// Surface treatments use the existing painted floor texture underneath, with
// a slight irregular edge and small, fixed organic fragments on the humus.
const humusFragments = [
  [5, 11, 4, 2], [12, 19, 6, 2], [18, 7, 5, 2], [28, 16, 5, 3],
  [33, 24, 4, 2], [36, 9, 5, 3], [21, 22, 3, 2],
] as const;

export function paintForestSubstrate(ctx: CanvasRenderingContext2D, cells: readonly ForestSubstrateCell[] | undefined) {
  for (const cell of cells ?? []) {
    if (cell.substrate === 'native') continue;
    const ground = forestSoilCell(cell.cellIndex);
    const humus = cell.substrate === 'coarseHumus';
    ctx.save();
    ctx.translate(ground.x * forestWidth, ground.y * forestHeight);
    ctx.scale(ground.width * forestWidth / 44, ground.height * forestHeight / 29);
    ctx.beginPath();
    ctx.moveTo(4, 7);
    ctx.lineTo(11, 3);
    ctx.lineTo(20, 5);
    ctx.lineTo(29, 2);
    ctx.lineTo(41, 7);
    ctx.lineTo(44, 13);
    ctx.lineTo(39, 22);
    ctx.lineTo(31, 24);
    ctx.lineTo(24, 29);
    ctx.lineTo(14, 25);
    ctx.lineTo(4, 23);
    ctx.lineTo(0, 17);
    ctx.closePath();
    ctx.clip();
    ctx.globalCompositeOperation = humus ? 'multiply' : 'soft-light';
    const tint = ctx.createRadialGradient(22, 15, 3, 22, 15, 25);
    tint.addColorStop(0, humus ? '#795742de' : '#cfa67cca');
    tint.addColorStop(.70, humus ? '#815c43af' : '#c7a1789a');
    tint.addColorStop(1, '#00000000');
    ctx.fillStyle = tint;
    ctx.fillRect(0, 0, 44, 29);
    if (humus) {
      ctx.globalCompositeOperation = 'source-over';
      const reverse = cell.cellIndex % 2 === 0;
      for (const [x, y, width, height] of humusFragments) {
        const left = reverse ? 44 - x - width : x;
        ctx.fillStyle = '#36281b8f';
        ctx.fillRect(left, y, width, height);
        ctx.fillStyle = '#ad805369';
        ctx.fillRect(left + 1, y, width - 2, 1);
      }
    }
    ctx.restore();
  }
}
