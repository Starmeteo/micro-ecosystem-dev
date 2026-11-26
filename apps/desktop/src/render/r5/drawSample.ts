import { R5_WIDTH, R5_HEIGHT, type Point, type Rect, type R5SampleTheme, type R5SampleObject, sampleProjection, point } from './geometry';
import {
  type ArtObject, type SampleImages, type SamplePlacements, type SceneArt,
  drawArtAt, localPoint, placedArt, sampleAnimal,
} from './sceneArt';

type Animal = ReturnType<typeof sampleAnimal>;

type SampleDraw = {
  theme: R5SampleTheme;
  art: SceneArt;
  images: SampleImages;
  placements: SamplePlacements;
  animal: Animal;
  selected: R5SampleObject | null;
  water: Rect | null;
};

function drawLayers(ctx: CanvasRenderingContext2D, frame: SampleDraw) {
  const { art, images, placements, animal } = frame;
  const image = (path?: string) => path ? images.get(path) : undefined;

  function drawSprite(object: ArtObject, position: Point, path?: string, scale = 1) {
    drawArtAt(ctx, images, object, position, path, scale);
  }

  function drawFixed(object?: ArtObject) {
    if (object) drawSprite(object, point(object.position), object.sprites.default);
  }

  function drawAnimal() {
    if (animal.attached || frame.theme === 'forest') {
      ctx.fillStyle = frame.theme === 'forest' ? 'rgba(16,20,12,.40)' : 'rgba(3,18,24,.37)';
      ctx.beginPath();
      ctx.ellipse(animal.position.x, animal.position.y + 1, 6, 1.5, 0, 0, Math.PI * 2);
      ctx.fill();
    }
    drawSprite(animal.animal, animal.position, animal.sprite, animal.scale);
  }

  ctx.drawImage(image(art.layers.base)!, 0, 0);
  drawFixed(art.objects.leaf);

  const supports: R5SampleObject[] = ['stone', 'log'];
  supports.sort((a, b) => placements[a].y - placements[b].y);
  for (const name of supports) {
    const support = placedArt(art, name);
    drawSprite(support, placements[name], support.sprites.shadow);
  }
  for (const name of supports) {
    const support = placedArt(art, name);
    const position = placements[name];
    drawSprite(support, position, support.sprites.back ?? support.sprites.default);
    if (name === 'log') {
      const moss = art.objects.moss;
      if (moss) {
        const [x, y] = support.position;
        drawSprite(moss, { x: position.x - x + moss.position[0], y: position.y - y + moss.position[1] }, moss.sprites.default);
      }
      if (animal.attached && frame.theme === 'forest') drawAnimal();
    }
    drawSprite(support, position, support.sprites.front);
    drawSprite(support, position, support.sprites.growth);
    if (name === 'log' && animal.attached && frame.theme === 'pond') drawAnimal();
  }

  drawFixed(art.objects.plant);
  if (!animal.attached) drawAnimal();

  if (frame.water && art.waterline !== undefined) {
    const { x, y, width, height } = frame.water;
    const tint = ctx.createLinearGradient(0, y, 0, y + height);
    tint.addColorStop(0, 'rgba(61,153,162,.16)');
    tint.addColorStop(1, 'rgba(12,54,80,.24)');
    ctx.save();
    ctx.beginPath();
    ctx.rect(x, y, width, height);
    ctx.clip();
    ctx.fillStyle = tint;
    ctx.fillRect(x, y, width, height);
    const light = image(art.layers.light);
    if (light) ctx.drawImage(light, 0, 0);
    ctx.restore();
    const surface = image(art.layers.surface);
    if (surface) ctx.drawImage(surface, 0, y - art.waterline);
  } else {
    const light = image(art.layers.light);
    if (light) ctx.drawImage(light, 0, 0);
  }

  drawFixed(art.objects.leafLarge);
  const foreground = image(art.layers.foreground);
  if (foreground) ctx.drawImage(foreground, 0, 0);
}

function drawContact(ctx: CanvasRenderingContext2D, frame: SampleDraw) {
  if (!frame.selected) return;
  const definition = placedArt(frame.art, frame.selected);
  const position = frame.placements[frame.selected];
  const origin = localPoint(definition, [0, 0], position);
  ctx.lineWidth = 1;
  ctx.strokeStyle = 'rgba(223,241,246,.74)';
  ctx.setLineDash([3, 3]);
  ctx.strokeRect(origin.x + .5, origin.y + .5, definition.width - 1, definition.height - 1);
  ctx.setLineDash([]);

  const contacts = definition.contact.map((contact) => localPoint(definition, contact, position));
  if (contacts.length < 2) return;
  ctx.beginPath();
  contacts.forEach(({ x, y }, index) => {
    if (index === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.strokeStyle = '#a3f3db';
  ctx.stroke();
}

export function drawSampleBuffer(canvas: HTMLCanvasElement, frame: SampleDraw) {
  const ctx = canvas.getContext('2d', { alpha: false });
  if (!ctx) return;
  const scale = canvas.width / R5_WIDTH;
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, R5_WIDTH, R5_HEIGHT);
  drawLayers(ctx, frame);
  drawContact(ctx, frame);
}

function viewportPixel(at: Point, viewport: Rect, output: HTMLCanvasElement): Point {
  return {
    x: (at.x - viewport.x) * output.width / viewport.width,
    y: (at.y - viewport.y) * output.height / viewport.height,
  };
}

export function drawRuler(
  ctx: CanvasRenderingContext2D, viewport: Rect, output: HTMLCanvasElement,
  referenceWidthMm: number, dpr: number, axis?: readonly [Point, Point],
) {
  ctx.save();
  ctx.lineWidth = Math.max(1, dpr);
  if (axis) {
    const [from, to] = axis.map((p) => viewportPixel(p, viewport, output));
    ctx.strokeStyle = '#9bf5d9';
    ctx.fillStyle = '#9bf5d9';
    ctx.beginPath();
    ctx.moveTo(from.x, from.y);
    ctx.lineTo(to.x, to.y);
    ctx.stroke();
    for (const { x, y } of [from, to]) ctx.fillRect(x - dpr, y - dpr, 2 * dpr, 2 * dpr);
  }

  const mm = 50;
  const left = 12 * dpr;
  const bottom = output.height - 13 * dpr;
  const ruler = sampleProjection(mm, referenceWidthMm, output.width, R5_WIDTH / viewport.width);
  ctx.fillStyle = 'rgba(10,21,26,.75)';
  ctx.fillRect(left - 5 * dpr, bottom - 22 * dpr, ruler + 10 * dpr, 31 * dpr);
  ctx.strokeStyle = '#e7faf2';
  ctx.fillStyle = '#e7faf2';
  ctx.font = `${11 * dpr}px system-ui, sans-serif`;
  ctx.textAlign = 'center';
  ctx.fillText(`${mm} mm`, left + ruler / 2, bottom - 6 * dpr);
  ctx.beginPath();
  ctx.moveTo(left, bottom - 3 * dpr);
  ctx.lineTo(left, bottom + 2 * dpr);
  ctx.lineTo(left + ruler, bottom + 2 * dpr);
  ctx.lineTo(left + ruler, bottom - 3 * dpr);
  ctx.stroke();
  ctx.restore();
}

export function presentScene(output: HTMLCanvasElement, logical: HTMLCanvasElement, viewport: Rect) {
  const ctx = output.getContext('2d', { alpha: false });
  if (!ctx) return;
  const factor = logical.width / R5_WIDTH;
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(logical, viewport.x * factor, viewport.y * factor, viewport.width * factor, viewport.height * factor,
    0, 0, output.width, output.height);
  return ctx;
}

export function presentSample(
  output: HTMLCanvasElement, logical: HTMLCanvasElement, viewport: Rect,
  frame: SampleDraw, showScale: boolean, dpr: number,
) {
  const ctx = presentScene(output, logical, viewport);
  if (!ctx || !showScale) return;
  const { animal } = frame;
  const [a, b] = animal.animal.body;
  const axis = [localPoint(animal.animal, a, animal.position, animal.scale),
    localPoint(animal.animal, b, animal.position, animal.scale)] as const;
  drawRuler(ctx, viewport, output, frame.art.referenceWidthMm, dpr, axis);
}
