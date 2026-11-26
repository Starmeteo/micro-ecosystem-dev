type Rgb = readonly [red: number, green: number, blue: number];

const winter: Rgb = [222, 235, 255];
const anchors: readonly { day: number; foliage: Rgb; light: Rgb }[] = [
  { day: 1, foliage: [216, 222, 234], light: winter },
  { day: 58, foliage: [222, 230, 239], light: winter },
  { day: 98, foliage: [229, 255, 227], light: [246, 255, 247] },
  { day: 193, foliage: [242, 255, 222], light: [255, 251, 231] },
  { day: 285, foliage: [255, 220, 168], light: [255, 244, 219] },
  { day: 366, foliage: [216, 222, 234], light: winter },
];

export function seasonColor(day: number | undefined, part: 'foliage' | 'light'): Rgb {
  if (day === undefined) return [255, 255, 255];
  const right = Math.max(1, anchors.findIndex((anchor) => anchor.day >= day));
  const start = anchors[right - 1];
  const end = anchors[right];
  const position = Math.max(0, Math.min(1, (day - start.day) / (end.day - start.day)));
  const mix = (index: 0 | 1 | 2) => Math.round(start[part][index] + (end[part][index] - start[part][index]) * position);
  return [mix(0), mix(1), mix(2)];
}
