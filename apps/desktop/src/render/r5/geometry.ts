export const R5_WIDTH = 480;
export const R5_HEIGHT = 270;

export type R5SampleTheme = 'forest' | 'pond';
export type R5SampleDensity = 240 | 480 | 960;
export type R5SampleZoom = 1 | 2 | 4;
export type R5SampleFocus = 'center' | 'contact' | 'animal';
export type R5SampleObject = 'stone' | 'log';
export type Point = { x: number; y: number };
export type Rect = Point & { width: number; height: number };

export function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

export function point([x, y]: readonly [number, number]): Point {
  return { x, y };
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

export function sampleViewport(zoom: R5SampleZoom, focus: Point): Rect {
  const width = R5_WIDTH / zoom;
  const height = R5_HEIGHT / zoom;
  return {
    x: clamp(focus.x - width / 2, 0, R5_WIDTH - width),
    y: clamp(focus.y - height / 2, 0, R5_HEIGHT - height),
    width,
    height,
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

export function previewPosition(position: Point, region: Rect): Point {
  return {
    x: Math.round(clamp(position.x, region.x, region.x + region.width)),
    y: Math.round(clamp(position.y, region.y, region.y + region.height)),
  };
}

export function scenePixels(normalized: Point): Point {
  return { x: normalized.x * R5_WIDTH, y: normalized.y * R5_HEIGHT };
}

export function sceneNormalised(position: Point): Point {
  return { x: position.x / R5_WIDTH, y: position.y / R5_HEIGHT };
}

export function insideScene(position: Point) {
  return position.x >= 0 && position.x <= R5_WIDTH && position.y >= 0 && position.y <= R5_HEIGHT;
}
