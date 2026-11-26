"""Paint two fixed F06 litter arrangements and their display-only depletion frames."""

from json import loads
from math import cos, sin
from pathlib import Path

from PIL import Image, ImageDraw


OUT = Path(__file__).resolve().parents[1] / 'assets' / 'r5'
PAINT = (
    ('#4f3828', '#8b5736', '#ad774c', '#c49260', '#775139'),
    ('#483728', '#765238', '#986c45', '#a98457', '#674b36'),
    ('#55432b', '#916c43', '#a47b4b', '#be985d', '#846543'),
    ('#41382b', '#6c5840', '#836b4c', '#9b805c', '#5d4b38'),
)
LEAF_EDGE = (
    (0, 0), (.12, -.38), (.24, -.51), (.37, -.82), (.44, -.72), (.57, -.93),
    (.68, -.56), (.76, -.57), (.88, -.29), (1, 0), (.88, .13), (.76, .32),
    (.66, .31), (.57, .52), (.45, .40), (.31, .52), (.19, .32), (.10, .27),
)


def leaf(draw, center, length, breadth, angle, timbre, worn=False):
    dark, body, light, fold, seam = PAINT[timbre]
    ca, sa = cos(angle), sin(angle)
    cx, cy = center

    def at(x, y):
        u, v = (x - .5) * length, y * breadth
        return round(cx + u * ca - v * sa), round(cy + u * sa + v * ca)

    edge = LEAF_EDGE
    if worn:
        edge = (*edge[:5], (.48, -.37), (.59, -.71), *edge[7:12], (.68, .1), *edge[13:])
    draw.polygon([at(x, y) for x, y in edge], fill=dark)
    draw.polygon([at(x, y) for x, y in (
        (.04, 0), (.16, -.29), (.36, -.49), (.44, -.56), (.50, -.29), (.58, -.60),
        (.71, -.44), (.95, 0), (.73, .15), (.55, .29), (.32, .30), (.16, .14),
    )], fill=body)
    draw.polygon([at(x, y) for x, y in (
        (.47, -.18), (.58, -.46), (.70, -.34), (.82, -.23), (.92, 0),
        (.73, .04), (.62, -.09),
    )], fill=light)
    draw.line([at(x, y) for x, y in ((.02, .07), (.26, .02), (.48, -.10), (.67, -.02), (.91, -.01))], fill=fold)
    for x, y in ((.22, -.25), (.39, -.53), (.55, -.26), (.68, -.35), (.79, -.20)):
        draw.line((at(x, y), at(x + .12, 0)), fill=seam)
    draw.line((at(-.10, .19), at(.06, .06)), fill=dark)


def litter_frame(spec, scale, name, phase):
    size = spec['width'], spec['height']
    ax, ay = spec['anchor']
    image = Image.new('RGBA', size)
    draw = ImageDraw.Draw(image)

    def add(dx, dy, mm, breadth, angle, palette, worn=False):
        leaf(draw, (ax + dx, ay + dy), mm * scale, breadth, angle, palette, worn)

    if phase == 'depleted':
        # The residual position marker is a paint convention for an existing
        # zero-stock patch. It has no corresponding leaf count or mass.
        radius = 12 if name == 'heap' else 8
        draw.line(((ax - radius, ay + 1), (ax - 2, ay - 3), (ax + radius, ay - 1)), fill='#6b5b46')
        draw.line(((ax - 6, ay + 3), (ax + 2, ay + 1), (ax + 7, ay + 3)), fill='#88765a')
        draw.point((ax + 11, ay + 1), fill='#a18a68')
        return image

    if phase == 'trace':
        add(-8 if name == 'heap' else -6, 0, 18, 5, -.18, 3, True)
        add(13 if name == 'heap' else 6, 2, 11, 4, .37, 1, True)
        return image

    if name == 'leaves':
        if phase == 'default':
            add(8, 2, 29, 9, .62, 1)
            add(0, -2, 55, 12, -.18, 0)
        else:
            add(-2, 0, 42, 9, -.17, 3, True)
            add(17, 4, 13, 4, .43, 1, True)
    elif phase == 'default':
        add(-24, 1, 53, 12, .34, 1)
        add(17, 4, 52, 13, -.30, 1)
        add(29, -4, 48, 10, -.20, 2)
        add(-23, -5, 52, 13, -.21, 0)
        add(3, 3, 45, 12, .17, 0)
    else:
        add(-21, 1, 36, 9, -.23, 3, True)
        add(20, 3, 28, 7, .43, 1, True)
        add(0, -2, 42, 10, -.29, 1, True)
    return image


def main():
    manifest = loads((OUT / 'manifest.json').read_text(encoding='utf-8'))
    forest = manifest['forest']
    scale = manifest['canvas']['width'] / forest['referenceWidthMm']
    for name in ('leaves', 'heap'):
        spec = forest['leafPatches'][name]
        previous_alpha = float('inf')
        for phase, path in spec['sprites'].items():
            sprite = litter_frame(spec, scale, name, phase)
            alpha = sprite.getchannel('A')
            assert alpha.getbbox() and alpha.getextrema() == (0, 255)
            opaque = alpha.histogram()[255]
            assert 0 < opaque < previous_alpha, (name, phase, opaque, previous_alpha)
            previous_alpha = opaque
            if phase == 'default':
                left, _, right, _ = alpha.getbbox()
                assert abs(right - left - spec['lengthMm'] * scale) < 7, (name, left, right)
            sprite.save(OUT / path, optimize=True)
        print(f'{name}: four decreasing but pickable authored frames')


if __name__ == '__main__':
    main()
