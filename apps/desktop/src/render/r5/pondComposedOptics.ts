// Bottom texture, water light and reflection are independent passes from the approved V2 composition.
export interface PondOpticsArt {
  bed: HTMLImageElement;
  caustics: HTMLImageElement;
  surface: HTMLImageElement;
  mirror: HTMLCanvasElement;
}
const width = 576;
const height = 324;
const bedStart = 197;
const farWater = '#093d3d';

function brightness(value: number) {
  return Number.isFinite(value) ? Math.max(0, Math.min(value, 1)) : 1;
}

function gradient(context: CanvasRenderingContext2D, x0: number, y0: number, x1: number, y1: number, stops: [number, string][]) {
  const colors = context.createLinearGradient(x0, y0, x1, y1);
  for (const [position, color] of stops) colors.addColorStop(position, color);
  return colors;
}

function pool(context: CanvasRenderingContext2D, x: number, y: number, radius: number, squash: number, rgb: string, opacity: number) {
  context.save();
  context.translate(x, y);
  context.scale(1, squash);
  const smooth = context.createRadialGradient(0, 0, 0, 0, 0, radius);
  smooth.addColorStop(0, 'rgba(' + rgb + ',' + opacity + ')');
  smooth.addColorStop(.48, 'rgba(' + rgb + ',' + opacity * .43 + ')');
  smooth.addColorStop(1, 'transparent');
  context.fillStyle = smooth;
  context.fillRect(-radius, -radius, radius * 2, radius * 2);
  context.restore();
}

function paintBed(context: CanvasRenderingContext2D, bed: HTMLImageElement, level: number, paintHeight: number) {
  const depth = height - bedStart;
  context.drawImage(bed, 0, bedStart, width, depth);
  if (paintHeight > height) {
    // Continue the independently painted sand beyond the close camera's bottom edge.
    context.save();
    context.beginPath();
    context.rect(0, height, width, paintHeight - height);
    context.clip();
    context.translate(0, height * 2);
    context.scale(1, -1);
    context.drawImage(bed, 0, bedStart, width, depth);
    context.restore();
  }
  // Keep the compressed distant crests in teal haze while the front relief stays warm and readable.
  context.fillStyle = gradient(context, 0, bedStart, 0, height, [
    [0, farWater], [.11, 'rgba(9,63,64,.92)'], [.25, 'rgba(9,58,56,.66)'],
    [.50, 'rgba(9,48,41,.22)'], [.76, 'rgba(13,38,35,.035)'], [1, 'rgba(33,42,29,.07)'],
  ]);
  context.fillRect(0, bedStart, width, paintHeight - bedStart);

  context.fillStyle = gradient(context, 0, 0, width, 0, [
    [0, 'rgba(6,32,34,.36)'], [.18, 'rgba(8,44,42,.24)'], [.36, 'rgba(11,45,37,.12)'],
    [.55, 'rgba(8,33,30,0)'], [.76, 'rgba(10,36,35,.16)'], [1, 'rgba(3,26,31,.41)'],
  ]);
  context.fillRect(0, 0, width, paintHeight);

  if (level < 1) {
    context.fillStyle = gradient(context, 0, bedStart, 0, height, [
      [0, 'rgba(3,19,24,0)'], [.26, 'rgba(4,23,30,' + (1 - level) * .25 + ')'],
      [.59, 'rgba(5,22,28,' + (1 - level) * .36 + ')'], [1, 'rgba(6,19,27,' + (1 - level) * .42 + ')'],
    ]);
    context.fillRect(0, bedStart, width, paintHeight - bedStart);
  }
}

export function paintPondBase(context: CanvasRenderingContext2D, images: PondOpticsArt, value = 1, bottom = height) {
  const level = brightness(value);
  const paintHeight = Math.max(height, bottom);
  context.save();
  context.imageSmoothingEnabled = true;
  context.imageSmoothingQuality = 'high';
  context.fillStyle = gradient(context, 0, 0, 0, height, [
    [0, '#063439'], [.15, '#105655'], [.39, '#0a4b48'],
    [bedStart / height, farWater], [.85, '#083636'], [1, '#092f32'],
  ]);
  context.fillRect(0, 0, width, paintHeight);
  paintBed(context, images.bed, level, paintHeight);

  context.globalAlpha *= .23 + .77 * level;
  pool(context, 134, 40, 199, .94, '90,157,115', .29);
  pool(context, 291, 122, 175, .65, '45,124,91', .19);
  pool(context, 481, 147, 161, 1.27, '0,25,40', .36);
  context.restore();
}

const shafts = [
  [98, 4, 146, 158, 4, 18, .095],
  [113, 1, 190, 199, 7, 21, .155],
  [141, 4, 206, 123, 3, 12, .115],
  [157, 5, 252, 173, 6, 23, .126],
  [181, 7, 281, 158, 4, 10, .071],
];

function paintShafts(context: CanvasRenderingContext2D, level: number) {
  context.save();
  context.globalCompositeOperation = 'screen';
  context.globalAlpha *= level;
  context.filter = 'blur(2.1px)';
  for (const [x0, y0, x1, y1, start, end, opacity] of shafts) {
    context.fillStyle = gradient(context, x0, y0, x1, y1, [
      [0, 'rgba(249,232,173,0)'], [.13, 'rgba(235,221,163,' + opacity + ')'],
      [.43, 'rgba(170,196,146,' + opacity * .53 + ')'],
      [.77, 'rgba(89,157,127,' + opacity * .135 + ')'], [1, 'rgba(64,132,121,0)'],
    ]);
    context.beginPath();
    context.moveTo(x0 - start, y0);
    context.lineTo(x0 + start, y0);
    context.lineTo(x1 + end, y1);
    context.lineTo(x1 - end, y1);
    context.closePath();
    context.fill();
  }
  context.restore();
}

const groups = [
  [343, 220, 25, .23], [326, 226, 30, .28], [346, 232, 42, .37],
  [320, 241, 53, .49], [337, 247, 57, .58], [307, 255, 62, .66],
  [325, 263, 79, .74], [319, 277, 82, .84], [291, 285, 90, .96],
  [322, 297, 113, .93], [300, 312, 125, 1],
];

function jitter(seed: number) {
  const value = Math.sin(seed * 91.53 + 83.71) * 17941.135;
  return value - Math.floor(value);
}

function flare(context: CanvasRenderingContext2D, x: number, y: number, span: number, teeth: number[], hot: number) {
  const [left, crest, tip] = teeth;
  context.beginPath();
  context.moveTo(x - span * .54, y + left * .32);
  context.lineTo(x - span * .35, y + left);
  context.lineTo(x - span * .16, y + crest * .08);
  context.lineTo(x + span * .04, y + crest);
  context.lineTo(x + span * .22, y + tip * .25);
  context.lineTo(x + span * .50, y + tip);
  context.strokeStyle = 'rgba(245,127,23,' + hot * .48 + ')';
  context.lineWidth = 2.1 + span * .041;
  context.stroke();
  context.strokeStyle = 'rgba(255,164,39,' + hot * .77 + ')';
  context.lineWidth = 1.4 + span * .025;
  context.stroke();

  context.beginPath();
  context.moveTo(x - span * .38, y + left);
  context.lineTo(x - span * .15, y + crest * .09);
  context.lineTo(x + span * .04, y + crest);
  context.lineTo(x + span * .22, y + tip * .25);
  context.lineTo(x + span * .34, y + tip * .58);
  context.strokeStyle = 'rgba(255,232,155,' + hot + ')';
  context.lineWidth = .47 + span * .018;
  context.stroke();
  if (hot > .55) {
    context.fillStyle = 'rgba(255,252,222,' + hot * .96 + ')';
    context.fillRect(x + span * .015, y + crest - .30, 1.0 + span * .036, .7 + span * .023);
  }
}

function sharpRefractions(context: CanvasRenderingContext2D, level: number) {
  context.save();
  context.globalAlpha *= level;
  context.globalCompositeOperation = 'screen';
  context.lineCap = 'butt';
  context.lineJoin = 'bevel';
  groups.forEach(([center, y, radius, depth], row) => {
    const count = row < 3 ? 4 : row < 8 ? 6 : 7;
    for (let mark = 0; mark < count; mark++) {
      const seed = 17 + row * 41 + mark * 5;
      const across = ((mark + .2 + .53 * jitter(seed)) / count - .5) * 2;
      const strength = (.77 - .28 * Math.abs(across)) * (depth * .87 + .11) * (.80 + .42 * jitter(seed + 3));
      const span = (4 + 12 * depth) * (.54 + .87 * jitter(seed + 1));
      const jag = .65 + 1.65 * depth;
      const teeth = [2, 4, 6].map(offset => (jitter(seed + offset) - .5) * 2 * jag);
      const x = center + radius * across;
      const at = y + (jitter(seed + 9) - .5) * (1.6 + 7.5 * depth);
      flare(context, x, at, span, teeth, strength);
      if (depth > .61 && jitter(seed + 11) > .46) {
        flare(context, x + span * .56, at + 1.4, span * .37, [0, -.58, 1], strength * .53);
      }
    }
  });
  context.restore();
}

export function paintPondLight(context: CanvasRenderingContext2D, images: PondOpticsArt, value = 1) {
  const level = brightness(value);
  if (level <= .015) return;
  paintShafts(context, level ** 1.17);
  context.save();
  context.globalCompositeOperation = 'screen';
  context.globalAlpha *= level ** 1.17;
  pool(context, 323, 273, 135, .48, '235,148,49', .17);
  if (images.caustics) {
    const patch = images.caustics;
    const w = patch.naturalWidth || patch.width;
    const h = patch.naturalHeight || patch.height;
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = 'high';
    context.globalAlpha *= .59;
    context.filter = 'saturate(1.05)';
    context.drawImage(patch, w * .025, h * .483, w * .965, h * .498, 163, 213, 314, 112);
  }
  context.restore();
  sharpRefractions(context, level ** 1.28);
}

export function paintPondSurface(context: CanvasRenderingContext2D, images: PondOpticsArt, value = 1) {
  const level = brightness(value);
  context.save();
  context.beginPath();
  context.rect(0, 0, width, 73);
  context.clip();
  if (images.mirror) {
    context.save();
    context.globalCompositeOperation = 'source-over';
    context.globalAlpha *= .046 + .87 * level ** 1.18;
    context.drawImage(images.mirror, 0, 0, width, 71);
    context.restore();
  }
  context.globalCompositeOperation = 'screen';
  context.fillStyle = gradient(context, 0, 0, 0, 55, [
    [0, 'rgba(100,153,116,' + (.022 + .09 * level) + ')'], [1, 'rgba(100,153,116,0)'],
  ]);
  context.fillRect(0, 0, width, 55);
  if (images.surface) {
    const image = images.surface;
    const w = image.naturalWidth || image.width;
    const h = image.naturalHeight || image.height;
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = 'high';
    context.globalAlpha *= .068 + (images.mirror ? .48 : .87) * level ** .72;
    context.filter = 'sepia(.38) saturate(1.15)';
    context.drawImage(image, w * .012, h * .148, w * .978, h * .57, -2, -4, width + 4, 64);
  }
  context.restore();
}
