"""Build independent P04 sand decals from sediment contours and in-scale grain facets."""

from dataclasses import dataclass
from math import sin, tau
from pathlib import Path
from random import Random

from PIL import Image, ImageDraw, ImageFilter


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5' / 'pond-composed'
PIXELS_PER_MM = 4
SIZE = (400, 104)
Point = tuple[float, float]
Color = tuple[int, int, int]


@dataclass(frozen=True)
class Sand:
    name: str
    contour: tuple[Point, ...]
    color: Color
    grain: tuple[float, float]
    spacing: float
    ridge: float
    seed: int
    light_grains: tuple[Color, ...]


FINE = Sand(
    'sand-fine-100',
    ((-48, 1), (-42, -3), (-32, -5.8), (-19, -5), (-10, -8.2), (4, -8.8),
     (17, -6.3), (25, -3.9), (35, -4), (42, -1.8), (48, 1.1), (38, 4.8),
     (27, 5.6), (16, 8.1), (4, 7.3), (-8, 4.9), (-21, 6.7), (-34, 5.5), (-43, 4.1)),
    (179, 151, 107), (.26, .76), .54, 6, 412,
    ((203, 179, 135), (192, 163, 120), (137, 108, 73), (186, 154, 110)),
)
COARSE = Sand(
    'sand-coarse-100',
    ((-48, -1.9), (-42, -5), (-36, -4.3), (-25, -7), (-10, -7.4), (-2, -5.4),
     (8, -8.8), (23, -7.9), (32, -5.4), (39, -2.6), (48, .1), (40, 3),
     (31, 3.8), (23, 7.9), (9, 8.8), (-3, 6.8), (-17, 7), (-26, 4.1), (-40, 3.1)),
    (144, 126, 94), (.74, 1.48), .94, 4.8, 701,
    ((192, 173, 131), (170, 144, 106), (104, 99, 79), (155, 137, 107)),
)


def pixels(x: float, y: float) -> Point:
    return SIZE[0] / 2 + x * PIXELS_PER_MM, SIZE[1] / 2 + y * PIXELS_PER_MM


def smooth_outline(contour: tuple[Point, ...]) -> list[Point]:
    def cubic(a: float, b: float, c: float, d: float, t: float) -> float:
        return b + .5 * t * (c - a + t * (2 * a - 5 * b + 4 * c - d + t * (3 * (b - c) + d - a)))

    closed = (contour[-1], *contour, contour[0], contour[1])
    points = []
    for index in range(len(contour)):
        a, b, c, d = closed[index:index + 4]
        for step in range(8):
            t = step / 8
            points.append(pixels(cubic(a[0], b[0], c[0], d[0], t), cubic(a[1], b[1], c[1], d[1], t)))
    return points


def footprint(material: Sand) -> Image.Image:
    outline = Image.new('L', SIZE)
    ImageDraw.Draw(outline).polygon(smooth_outline(material.contour), fill=255)
    # A short tapered margin blends into the existing bed without a straight sprite border.
    edge = outline.filter(ImageFilter.GaussianBlur(4))
    halo = outline.filter(ImageFilter.GaussianBlur(8))
    return Image.blend(edge, halo, .22).point(lambda alpha: round(alpha * .81))


def ridge_phase(x: float, y: float, spacing: float) -> float:
    return (y + 1.6 * sin(x / 13 + y * .09) + .58 * sin(x / 4.5 + 1)) / spacing * tau


def sediment(material: Sand) -> Image.Image:
    color = Image.new('RGB', SIZE)
    colors = []
    for row in range(SIZE[1]):
        y = (row - SIZE[1] / 2) / PIXELS_PER_MM
        for column in range(SIZE[0]):
            x = (column - SIZE[0] / 2) / PIXELS_PER_MM
            phase = ridge_phase(x, y, material.ridge)
            lamina = (4.8 * sin(phase) + 2.4 * sin(phase * 2 + .8)) * (.56 + .39 * sin(x / 19 + .8))
            warm_edge = 5.2 * sin(x / 21 + y / 15) + 2.1 * sin(x / 8.1 - y / 5.4)
            tone = lamina + warm_edge + .33 * y
            colors.append(tuple(round(channel + tone) for channel in material.color))
    color.putdata(colors)
    return color.convert('RGBA')


def facets(material: Sand, coverage: Image.Image) -> Image.Image:
    grains = Image.new('RGBA', SIZE)
    pen = ImageDraw.Draw(grains)
    rng = Random(material.seed)
    rows = round(22 / material.spacing)
    columns = round(98 / material.spacing)
    for row in range(rows):
        for column in range(columns):
            x = -49 + (column + rng.random()) * material.spacing
            y = -11 + (row + rng.random()) * material.spacing
            centre = pixels(x, y)
            if coverage.getpixel(tuple(round(value) for value in centre)) < 23:
                continue
            # Deposition follows the ridges rather than an even grain field.
            ridge = .5 + .5 * sin(ridge_phase(x, y, material.ridge) + .55)
            if rng.random() > .19 + .69 * ridge:
                continue
            radius = rng.uniform(*material.grain) / 2
            depth = radius * rng.uniform(.35, .63)
            lean = depth * rng.uniform(-.45, .45)
            contour = [(x - radius, y), (x - radius * .28 + lean, y - depth),
                       (x + radius * .61, y - depth * .48), (x + radius, y + depth * .24),
                       (x + radius * .11, y + depth), (x - radius * .59, y + depth * .55)]
            tint = material.light_grains[rng.randrange(len(material.light_grains))]
            opacity = rng.randint(57, 148)
            pen.polygon([pixels(*point) for point in contour], fill=(*tint, opacity))
            if radius > .48:
                pen.line([pixels(*contour[1]), pixels(*contour[2])], fill=(*material.light_grains[0], opacity // 2))
    return grains


def render(material: Sand) -> Image.Image:
    coverage = footprint(material)
    grain = facets(material, coverage)
    decal = Image.alpha_composite(sediment(material), grain)
    decal.putalpha(coverage)
    return decal


def main() -> None:
    for material in (FINE, COARSE):
        decal = render(material)
        assert decal.getchannel('A').getextrema() == (0, 207)
        decal.save(OUT / (material.name + '.png'), optimize=True)


if __name__ == '__main__':
    main()
