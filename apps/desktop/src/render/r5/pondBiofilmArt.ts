import { spritePoint, type SceneryStyle, type SpritePose } from './pondComposedArt';
import type { PondBiofilmSurface } from '../../world';

// Local patches follow the negative slope of the bark grain and the broad rock facets.
const patches = {
  wood: [
    [.12, .80, .105, .065, -.78], [.24, .68, .087, .052, -.76], [.38, .57, .11, .048, -.63],
    [.52, .48, .105, .061, -.59], [.64, .39, .097, .055, -.61], [.77, .31, .077, .046, -.73],
    [.88, .23, .085, .041, -.82],
  ],
  stone: [
    [.16, .64, .128, .16, -.52], [.31, .47, .105, .11, -.42], [.49, .62, .156, .13, -.53],
    [.61, .49, .131, .104, -.35], [.72, .73, .125, .13, -.62], [.86, .60, .114, .14, -.44],
  ],
} as const;

const surfaceArt = {
  wood: new WeakMap<SpritePose['art']['image'], HTMLCanvasElement>(),
  stone: new WeakMap<SpritePose['art']['image'], HTMLCanvasElement>(),
};

function filmTexture(kind: PondBiofilmSurface['kind'], { art }: SpritePose) {
  const cache = surfaceArt[kind];
  const cached = cache.get(art.image);
  if (cached) return cached;
  const canvas = document.createElement('canvas');
  canvas.width = art.mask.width;
  canvas.height = art.mask.height;
  const brush = canvas.getContext('2d');
  if (!brush) return;
  brush.scale(canvas.width, canvas.height);
  for (const [u, v, width, height, angle] of patches[kind]) {
    brush.save();
    brush.translate(u, v);
    brush.rotate(angle);
    brush.scale(width, height);
    const wash = brush.createRadialGradient(-.15, -.21, .08, 0, 0, 1);
    wash.addColorStop(0, '#89965bdf');
    wash.addColorStop(.49, '#73864ac3');
    wash.addColorStop(.76, '#667b4380');
    wash.addColorStop(1, '#667b4300');
    brush.fillStyle = wash;
    brush.fillRect(-1, -1, 2, 2);
    brush.fillStyle = '#a5ab6cc2';
    for (const [x, y, length] of [[-.46, -.26, .073], [-.28, .03, .042], [-.03, -.22, .085],
      [.18, .15, .055], [.39, -.10, .073], [.54, .24, .052], [.27, -.34, .035]]) {
      brush.fillRect(x, y, length, .028);
    }
    brush.restore();
  }
  brush.resetTransform();
  brush.globalCompositeOperation = 'destination-in';
  brush.drawImage(art.image, 0, 0, canvas.width, canvas.height);
  cache.set(art.image, canvas);
  return canvas;
}

export function drawPondSurfaceFilm(context: CanvasRenderingContext2D, style: SceneryStyle, pose: SpritePose,
  surface: PondBiofilmSurface, sun: number) {
  const texture = filmTexture(surface.kind, pose);
  if (!texture) return;
  const origin = spritePoint(pose, [0, 0]);
  const right = spritePoint(pose, [1, 0]);
  const bottom = spritePoint(pose, [0, 1]);
  const tint = surface.mgC / (surface.mgC + (surface.kind === 'wood' ? 6 : 2.4));
  context.save();
  context.globalAlpha *= (.24 + sun * .21) * tint * (style.opacity ?? 1);
  context.transform(right.x - origin.x, right.y - origin.y, bottom.x - origin.x, bottom.y - origin.y, origin.x, origin.y);
  if (style.rootCut) {
    context.beginPath();
    context.rect(0, 0, 1, Math.min(1, pose.art.anchor[1] + 1 / pose.art.h));
    context.clip();
  }
  context.drawImage(texture, 0, 0, 1, 1);
  context.restore();
}
