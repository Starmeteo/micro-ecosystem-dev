import type { PondBasinSnapshot, PondFloatingPlantPreset } from '../../world';
import { composedPixelsPerMm, composedWidth, type ComposedArt } from './pondComposedArt';
import type { Point } from './geometry';

const dimensions: Record<PondFloatingPlantPreset, { diameter: number; root: number }> = {
  small: { diameter: 30, root: 30 },
  medium: { diameter: 45, root: 60 },
  large: { diameter: 60, root: 100 },
};

// Opaque bounds and blade/root join of the independently authored floating-45 sprite.
const spriteBounds = { left: 420 / 1536, width: 686 / 1536, top: 144 / 1024, join: 422 / 1024, bottom: 865 / 1024 };

interface FloatingVisual extends Point {
  id: number;
  width: number;
  blade: number;
  roots: number;
}

export function pondFloatingVisuals(pond: PondBasinSnapshot, surface: number, bed: number): FloatingVisual[] {
  if (!pond.wet || !pond.floatingPlants?.length) return [];
  const waterColumn = Math.max(0, bed - surface);
  const surfaceBand = Math.min(42, waterColumn * .26);
  const { rows, protectedRow } = pond.shape ?? { rows: 12, protectedRow: 5.5 };
  const plants = pond.floatingPlants.flatMap(({ id, position, preset, diameterMm, floating }) => {
    if (!floating || !Number.isFinite(diameterMm) || diameterMm <= 0) return [];
    // Fixed observation row keeps the side projection stable when distant shoreline cells change.
    const depth = .5 + Math.atan((position.y * rows - protectedRow - .5) * .56) / Math.PI;
    const y = surface + surfaceBand * (.37 + depth * .38);
    const width = diameterMm * composedPixelsPerMm * (.89 + .11 * depth);
    const blade = Math.min(width * (.32 + .06 * depth), Math.max(3, surfaceBand * .52));
    const design = dimensions[preset];
    const roots = Math.max(0, Math.min(design.root * (diameterMm / design.diameter) * composedPixelsPerMm,
      pond.depthCm * 10 * composedPixelsPerMm, bed - y - blade * .12 - 3));
    return [{ id, x: position.x * composedWidth, y, width, blade, roots }];
  });
  return plants.sort((a, b) => a.y - b.y || a.id - b.id);
}

export function paintFloatingShade(context: CanvasRenderingContext2D, plants: readonly FloatingVisual[], sun: number) {
  if (!plants.length || sun <= 0) return;
  context.save();
  context.fillStyle = 'rgba(9, 38, 35,' + sun * .20 + ')';
  context.filter = 'blur(2.4px)';
  context.beginPath();
  for (const { x, y, width, roots } of plants) {
    const span = width * .48;
    const radiusY = Math.max(1, Math.min(16, roots * .22));
    const at = y + 3 + radiusY;
    context.moveTo(x + 3 + span, at);
    context.ellipse(x + 3, at, span, radiusY, 0, 0, Math.PI * 2);
  }
  context.fill();
  context.restore();
}

export function paintFloatingRoots(context: CanvasRenderingContext2D, art: ComposedArt, plants: readonly FloatingVisual[]) {
  const sprite = art.sprites.get('floating-45');
  if (!sprite || !plants.length) return;
  const { image, w, h } = sprite;
  const { left, width, join, bottom } = spriteBounds;
  context.save();
  context.globalAlpha *= .72;
  context.filter = 'brightness(.82) saturate(.73)';
  for (const plant of plants) if (plant.roots > 0) {
    context.drawImage(image, left * w, join * h, width * w, (bottom - join) * h,
      plant.x - plant.width / 2, plant.y + plant.blade * .12, plant.width, plant.roots);
  }
  context.restore();
}

export function paintFloatingBlades(context: CanvasRenderingContext2D, art: ComposedArt, plants: readonly FloatingVisual[], sun: number) {
  const sprite = art.sprites.get('floating-45');
  if (!sprite || !plants.length) return;
  const { image, w, h } = sprite;
  const { left, width, top, join } = spriteBounds;
  context.save();
  for (const plant of plants) {
    const bladeTop = plant.y - plant.blade * .88;
    const ringY = bladeTop + plant.blade * .54;
    context.strokeStyle = 'rgba(210, 237, 202,' + (.16 + .23 * sun) + ')';
    context.lineWidth = .8;
    context.beginPath();
    context.ellipse(plant.x, ringY, plant.width * .57, plant.blade * .53 + 1, 0, .08, Math.PI * .82);
    context.stroke();
    context.beginPath();
    context.ellipse(plant.x, ringY, plant.width * .57, plant.blade * .53 + 1, 0, 3.22, Math.PI * 1.84);
    context.stroke();
    context.drawImage(image, left * w, top * h, width * w, (join - top) * h,
      plant.x - plant.width / 2, bladeTop, plant.width, plant.blade);
  }
  context.restore();
}

export function markFloatingPlant(context: CanvasRenderingContext2D, plants: readonly FloatingVisual[], selectedId?: number) {
  const plant = plants.find(({ id }) => id === selectedId);
  if (!plant) return;
  context.save();
  context.strokeStyle = '#c0f0df';
  context.lineWidth = 1.3;
  context.beginPath();
  context.ellipse(plant.x, plant.y - plant.blade * .38,
    plant.width * .5 + 4, plant.blade * .5 + 3, 0, 0, Math.PI * 2);
  context.stroke();
  context.restore();
}
