export const R5_WIDTH = 480;
export const R5_HEIGHT = 270;

export type R5SampleZoom = 1 | 2 | 4;
export type R5SampleFocus = 'center' | 'contact' | 'animal';
export type Point = { x: number; y: number };
export type Rect = Point & { width: number; height: number };

export function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

export function fitSample(width: number, height: number, devicePixelRatio: number) {
  const fit = Math.max(0, Math.min(width / R5_WIDTH, height / R5_HEIGHT));
  return {
    cssWidth: R5_WIDTH * fit,
    cssHeight: R5_HEIGHT * fit,
    pixelWidth: Math.max(1, Math.floor(R5_WIDTH * fit * devicePixelRatio)),
    pixelHeight: Math.max(1, Math.floor(R5_HEIGHT * fit * devicePixelRatio)),
    devicePixelRatio,
  };
}

export function pointerOnSample(client: Point, canvas: Rect, viewport: Rect): Point {
  return {
    x: viewport.x + (client.x - canvas.x) * viewport.width / canvas.width,
    y: viewport.y + (client.y - canvas.y) * viewport.height / canvas.height,
  };
}

export function sampleProjection(lengthMm: number, referenceWidthMm: number, contentWidth: number, zoom = 1) {
  return lengthMm / referenceWidthMm * contentWidth * zoom;
}
