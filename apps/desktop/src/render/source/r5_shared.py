"""Drawing primitives for the independent R5 pixel cels."""

from math import cos, sin
from random import Random

from PIL import Image, ImageChops

from forest_art import Cel, frond


def layered_background(top, bottom, end):
    """Build the quiet part of the palette in broad, deliberate color bands."""
    cel = Cel(1)
    for y in range(0, 270, 3):
        blend = min(y / end, 1)
        rgb = tuple(round(a + (b - a) * blend) for a, b in zip(top, bottom))
        cel.box(rgb, 0, y, 480, 3)
    return cel


def blade(cel, start, control, end, width, side, fill, vein):
    """Draw a single curved leaf with stepped lobes and a narrowing midrib."""
    def along(t):
        a = 1 - t
        return (a * a * start[0] + 2 * a * t * control[0] + t * t * end[0],
                a * a * start[1] + 2 * a * t * control[1] + t * t * end[1])

    line = [along(i / 10) for i in range(11)]
    upper, lower = [], []
    for i, (x, y) in enumerate(line):
        d = width * sin(i / 10 * 3.14159) * (1 if i % 2 == 0 else .84)
        upper.append((x + d * side[0], y + d * side[1]))
        lower.append((x - d * side[0] * .75, y - d * side[1] * .75))
    cel.poly(fill, *upper, *reversed(lower))
    cel.line(vein, 1, *line[:-1])


def fallen_leaf(cel, start, length, width, angle, palette):
    dark, mid, gold, dry = palette
    ca, sa = cos(angle), sin(angle)

    def at(x, y):
        return start[0] + x * ca - y * sa, start[1] + x * sa + y * ca

    top = [(0, 0), (.13, -.43), (.25, -.55), (.27, -.71), (.40, -.73),
           (.44, -.92), (.58, -.75), (.63, -.91), (.75, -.61), (.78, -.68),
           (.89, -.31), (1, -.04)]
    bottom = [(.89, .18), (.84, .40), (.73, .44), (.70, .60), (.51, .52),
              (.45, .61), (.32, .45), (.26, .51), (.12, .23)]
    shape = [at(x * length, y * width) for x, y in top + bottom]
    cel.poly(dark, *shape)
    cel.poly(mid, *[at(x * length, y * width) for x, y in top[1:-1]],
             at(.98 * length, -.04 * width), at(.78 * length, .07 * width),
             at(.63 * length, -.14 * width), at(.46 * length, .08 * width), at(0, 0))
    cel.line(gold, 1, at(-length * .09, width * .15), at(.32 * length, 0),
             at(.64 * length, -width * .05), at(.96 * length, -width * .04))
    for j in range(1, 7):
        x = (.11 + .108 * j) * length
        h = width * sin(3.14159 * x / length)
        cel.line(gold if j % 4 == 1 else dry, 1,
                 at(x, 0), at(x + .056 * length, -.30 * h), at(x + .081 * length, -.62 * h))
        cel.line(dry, 1, at(x, 0), at(x + .06 * length, .20 * h), at(x + .08 * length, .32 * h))


def moss_pad(cel, x, y, width, height, palette, seed, pendant=False):
    """Use separate growing clusters and a quieter dark bound underneath."""
    rnd = Random(seed)
    dark, low, mid, rim = palette
    width = width / 2
    cel.poly(dark, (x - width, y), (x - .78 * width, y - .32 * height),
             (x - .54 * width, y - .77 * height), (x - .16 * width, y - .68 * height),
             (x + .15 * width, y - .94 * height), (x + .58 * width, y - .73 * height),
             (x + .88 * width, y - .45 * height), (x + width, y - .06 * height),
             (x + .71 * width, y + .14 * height), (x - .32 * width, y + .15 * height))
    for n in range(round(width * 2.2)):
        dx = rnd.uniform(-.97, .97)
        yy = y - rnd.uniform(.1, 1) * height * (1 - abs(dx) * .69)
        xx = x + dx * width
        bend = rnd.choice((-1, 1)) * rnd.uniform(.7, 2.6)
        tone = [low, mid, low, rim, mid, low][n % 6]
        if pendant:
            drop = rnd.uniform(height * .22, height * .65)
            cel.line(tone, 1, (xx - bend, yy - 1), (xx, yy + drop * .45), (xx + bend, yy + drop))
            cel.line(tone, 1, (xx, yy + drop * .45), (xx - bend * 1.4, yy + drop * .54))
        else:
            rise = rnd.uniform(1, 3.6)
            cel.line(tone, 1, (xx, yy + 2), (xx + bend * .38, yy), (xx + bend, yy - rise))
            if n % 3 == 0:
                cel.line(tone, 1, (xx, yy), (xx - bend, yy - rise * .5))


def fern_cluster(cel, root, targets, colors, width):
    dark, mid, rim = colors
    for index, target in enumerate(targets):
        x, y = root
        dx, dy = target[0] - x, target[1] - y
        curve = (x + dx * (.12 + .14 * (index % 3)), y + dy * (.75 + .09 * (index % 3)))
        frond(cel, (root, curve, target), width * (.69 + .15 * (index % 3)),
              (dark, dark, mid, rim), near=width > 8)


def rock_cel(size, outline, faces, texture, seed):
    cel = Cel(1, size)
    cel.poly(faces[0][0], *outline)
    for color, vertices in faces:
        cel.poly(color, *vertices)
    alpha = cel.image.getchannel('A')
    rnd = Random(seed)
    x0, y0, x1, y1 = alpha.getbbox()
    for i in range(85):
        x = rnd.randrange(x0 + 3, x1 - 3)
        y = rnd.randrange(y0 + 2, y1 - 5)
        w = rnd.uniform(1, 5) if i % 7 else rnd.uniform(6, 10)
        h = rnd.uniform(.6, 2.8)
        pigment = texture[i % len(texture)]
        under = cel.image.getpixel((x, y))
        if not under[3]:
            continue
        rgb = tuple(round(a * .73 + int(pigment[j:j + 2], 16) * .27)
                    for j, a in zip((1, 3, 5), under[:3]))
        cel.poly(rgb, (x, y), (x + w * .43, y - h),
                 (x + w, y - h * .6), (x + w * .71, y + 1), (x + 1, y + h))
    cel.image.putalpha(alpha)
    return cel


def ellipse_shadow(size, centered, width, height, tint):
    cel = Cel(1, size)
    x, y = centered
    for fraction, opacity in ((1, 28), (.85, 46), (.66, 78), (.48, 96)):
        w, h = width * fraction, height * fraction
        cel.oval((*tint, opacity), x - w / 2, y - h / 2, w, h)
    return cel


def add_clipped(target, overlay):
    """Clip detailed pigment to the existing painted silhouette."""
    overlay.image.putalpha(ImageChops.multiply(overlay.image.getchannel('A'), target.image.getchannel('A')))
    target.image.alpha_composite(overlay.image)


def split_paint(cel, vertices):
    """Return opaque tiles cut from one painted form, without duplicated edges."""
    cut = Cel(1, cel.image.size)
    cut.poly('white', *vertices)
    mask = cut.image.getchannel('A')
    alpha = cel.image.getchannel('A')
    front = Cel(1, cel.image.size)
    front.image.alpha_composite(cel.image)
    front.image.putalpha(ImageChops.multiply(alpha, mask))
    cel.image.putalpha(ImageChops.subtract(alpha, mask))
    return cel, front
